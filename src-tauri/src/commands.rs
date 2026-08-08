use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::ai::parse::{parse_daily_summary, parse_organization, validate_against_batch};
use crate::ai::prompt::{build_system_prompt, build_user_payload};
use crate::errors::{AppError, AppResult};
use crate::notes_fs::{generate_id, read_note_file, save_note_file, save_pasted_image, title_from_markdown};
use crate::state::AppState;
use crate::storage::categories as categories_db;
use crate::storage::notes as notes_db;
use crate::storage::suggestions as suggestions_db;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct SettingsDto {
    pub data_dir: String,
    pub configured: bool,
    pub hotkey: String,
    pub org_time: String,
    pub auto_org_enabled: bool,
    pub ai_base_url: String,
    pub ai_model: String,
    pub ai_api_key: String,
}

impl From<crate::config::Config> for SettingsDto {
    fn from(c: crate::config::Config) -> Self {
        Self {
            data_dir: c.data_dir.to_string_lossy().to_string(),
            configured: c.configured,
            hotkey: c.hotkey,
            org_time: c.org_time,
            auto_org_enabled: c.auto_org_enabled,
            ai_base_url: c.ai.base_url,
            ai_model: c.ai.model,
            ai_api_key: c.ai.api_key,
        }
    }
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> AppResult<SettingsDto> {
    let cfg = state.config.lock().unwrap().clone();
    Ok(cfg.into())
}

#[tauri::command]
pub fn update_settings(state: State<AppState>, s: SettingsDto) -> AppResult<()> {
    let mut cfg = state.config.lock().unwrap();
    cfg.hotkey = s.hotkey;
    cfg.org_time = s.org_time;
    cfg.auto_org_enabled = s.auto_org_enabled;
    cfg.ai.base_url = s.ai_base_url;
    cfg.ai.model = s.ai_model;
    cfg.ai.api_key = s.ai_api_key;
    let root = state.data_dir.lock().unwrap().clone();
    crate::config::save(&root, &cfg).map_err(AppError::new)?;
    Ok(())
}

#[tauri::command]
pub fn init_data_dir(state: State<AppState>, dir: String) -> AppResult<()> {
    let path = std::path::PathBuf::from(&dir);
    std::fs::create_dir_all(&path).map_err(AppError::from)?;
    let cfg = crate::config::load(&path).map_err(AppError::new)?;
    let storage = crate::storage::Storage::open(&path.join("quicknote.db"))?;
    let mut guard = state.config.lock().unwrap();
    guard.data_dir = path.clone();
    guard.configured = true;
    crate::config::save(&path, &guard).map_err(AppError::new)?;
    drop(guard);
    *state.storage.lock().unwrap() = storage;
    *state.data_dir.lock().unwrap() = path;
    Ok(())
}

#[tauri::command]
pub fn migrate_data_dir(state: State<AppState>, new_dir: String) -> AppResult<String> {
    let target = std::path::PathBuf::from(&new_dir);
    std::fs::create_dir_all(&target).map_err(AppError::from)?;
    let source = state.data_dir.lock().unwrap().clone();
    if source == target {
        return Ok(target.to_string_lossy().into_owned());
    }

    {
        let storage = state.storage.lock().unwrap();
        storage
            .conn()
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(AppError::from)?;
    }

    for name in ["config.json", "quicknote.db", "notes", "attachments"] {
        let from = source.join(name);
        if !from.exists() {
            continue;
        }
        copy_recursive(&from, &target.join(name)).map_err(AppError::from)?;
    }

    let cfg = crate::config::load(&target).map_err(AppError::new)?;
    let storage = crate::storage::Storage::open(&target.join("quicknote.db"))?;
    let mut guard = state.config.lock().unwrap();
    guard.data_dir = target.clone();
    guard.configured = true;
    crate::config::save(&target, &guard).map_err(AppError::new)?;
    drop(guard);
    *state.storage.lock().unwrap() = storage;
    *state.data_dir.lock().unwrap() = target.clone();
    Ok(target.to_string_lossy().into_owned())
}

fn copy_recursive(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    if from.is_dir() {
        std::fs::create_dir_all(to)?;
        for entry in std::fs::read_dir(from)? {
            let entry = entry?;
            copy_recursive(&entry.path(), &to.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        std::fs::copy(from, to)?;
        Ok(())
    }
}

#[derive(serde::Deserialize)]
pub struct PastedImage {
    pub filename: String,
    pub mime: String,
    pub bytes: Vec<u8>,
}

#[derive(Serialize)]
pub struct SaveNoteResult {
    pub id: String,
    pub markdown_path: String,
    pub image_refs: Vec<String>,
}

#[tauri::command]
pub fn save_note(
    state: State<AppState>,
    markdown: String,
    images: Vec<PastedImage>,
) -> AppResult<SaveNoteResult> {
    let now = chrono::Local::now();
    let id = generate_id(&now.fixed_offset());
    let created = now.to_rfc3339();
    let title = title_from_markdown(&markdown);

    let markdown_path = format!("notes/{}/{}/{}.md", &id[0..4], &id[4..6], id);
    let mut image_refs = Vec::new();
    let root = state.data_dir.lock().unwrap().clone();
    for img in images {
        let rel = save_pasted_image(&root, &id, &img.filename, &img.bytes, &img.mime)
            .map_err(AppError::new)?;
        image_refs.push(rel);
    }

    save_note_file(&root, &id, &markdown).map_err(AppError::new)?;
    let storage = state.storage.lock().unwrap();
    notes_db::insert_note(storage.conn(), &id, &title, &markdown, &created)?;

    Ok(SaveNoteResult {
        id,
        markdown_path,
        image_refs,
    })
}

#[tauri::command]
pub fn list_notes(
    state: State<AppState>,
    date: Option<String>,
) -> AppResult<Vec<notes_db::NoteMeta>> {
    let storage = state.storage.lock().unwrap();
    let rows = match date {
        Some(d) => notes_db::list_notes_by_date(storage.conn(), &d)?,
        None => notes_db::list_all(storage.conn())?,
    };
    Ok(rows)
}

#[tauri::command]
pub fn list_notes_by_category(
    state: State<AppState>,
    category_id: i64,
) -> AppResult<Vec<notes_db::NoteMeta>> {
    let storage = state.storage.lock().unwrap();
    Ok(notes_db::list_notes_by_category(storage.conn(), category_id)?)
}

#[tauri::command]
pub fn get_data_dir(state: State<AppState>) -> AppResult<String> {
    Ok(state
        .data_dir
        .lock()
        .unwrap()
        .to_string_lossy()
        .into_owned())
}

#[tauri::command]
pub fn get_note(state: State<AppState>, id: String) -> AppResult<Option<String>> {
    let storage = state.storage.lock().unwrap();
    let exists = notes_db::get_note(storage.conn(), &id)?;
    drop(storage);
    if exists.is_none() {
        return Ok(None);
    }
    let root = state.data_dir.lock().unwrap().clone();
    Ok(Some(read_note_file(&root, &id).map_err(AppError::new)?))
}

#[tauri::command]
pub fn update_note(state: State<AppState>, id: String, markdown: String) -> AppResult<()> {
    let storage = state.storage.lock().unwrap();
    if notes_db::get_note(storage.conn(), &id)?.is_none() {
        return Err(AppError::new("记录不存在"));
    }
    let root = state.data_dir.lock().unwrap().clone();
    save_note_file(&root, &id, &markdown).map_err(AppError::new)?;
    notes_db::refresh_body_index(storage.conn(), &id, &markdown)?;
    Ok(())
}

#[tauri::command]
pub fn save_image(
    state: State<AppState>,
    note_id: String,
    image: PastedImage,
) -> AppResult<String> {
    let storage = state.storage.lock().unwrap();
    if notes_db::get_note(storage.conn(), &note_id)?.is_none() {
        return Err(AppError::new("记录不存在"));
    }
    let root = state.data_dir.lock().unwrap().clone();
    let rel = save_pasted_image(
        &root,
        &note_id,
        &image.filename,
        &image.bytes,
        &image.mime,
    )
    .map_err(AppError::new)?;
    notes_db::set_ai_status(storage.conn(), &note_id, "pending")?;
    Ok(rel)
}

#[tauri::command]
pub fn rebuild_index(state: State<AppState>) -> AppResult<(usize, usize)> {
    let storage = state.storage.lock().unwrap();
    let root = state.data_dir.lock().unwrap().clone();
    crate::storage::rebuild::rebuild(storage.conn(), &root)
}

#[tauri::command]
pub fn list_categories(state: State<AppState>) -> AppResult<Vec<categories_db::Category>> {
    let storage = state.storage.lock().unwrap();
    Ok(categories_db::list(storage.conn())?)
}

#[derive(Serialize, Clone)]
pub struct OrgRunResult {
    pub processed: usize,
    pub suggested: usize,
    pub failed: Vec<String>,
}

/// 由命令与定时器共用的整理主流程。
pub async fn run_ai_org_inner(app: &AppHandle) -> AppResult<OrgRunResult> {
    let state = app.state::<AppState>();

    let (batch, categories, ai_config) = {
        let cfg = state.config.lock().unwrap();
        if cfg.ai.api_key.is_empty() {
            return Err(AppError::new("请先在设置中配置 API Key"));
        }
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let storage = state.storage.lock().unwrap();
        let batch = notes_db::pending_ids(storage.conn(), &date)?;
        let cats: Vec<String> = categories_db::list(storage.conn())?
            .into_iter()
            .filter(|c| c.enabled)
            .map(|c| c.name)
            .collect();
        (
            batch,
            cats,
            (
                cfg.ai.base_url.clone(),
                cfg.ai.model.clone(),
                cfg.ai.api_key.clone(),
            ),
        )
    };

    if batch.is_empty() {
        return Ok(OrgRunResult {
            processed: 0,
            suggested: 0,
            failed: vec![],
        });
    }

    let mut failed = Vec::new();
    let mut all_suggestions = Vec::new();
    let mut daily_summary: Option<String> = None;
    let root = state.data_dir.lock().unwrap().clone();

    for chunk in batch.chunks(30) {
        let mut notes_text: Vec<(String, String)> = Vec::new();
        for id in chunk {
            let text = read_note_file(&root, id).unwrap_or_else(|_| "[读取失败]".to_string());
            notes_text.push((id.clone(), text));
        }
        let system = build_system_prompt(&categories, 10);
        let user = build_user_payload(&notes_text, "请只输出 JSON");
        let mut attempt = 0;
        let parsed = loop {
            let raw = match state
                .ai
                .chat_json(&ai_config.0, &ai_config.1, &ai_config.2, &system, user.clone())
                .await
            {
                Ok(v) => v.to_string(),
                Err(e) => {
                    failed.push(format!("{}: {}", chunk[0], e.error));
                    break Vec::new();
                }
            };
            match parse_organization(&raw) {
                Ok(p) => {
                    if daily_summary.is_none() {
                        daily_summary = parse_daily_summary(&raw);
                    }
                    break p;
                }
                Err(_) if attempt == 0 => {
                    attempt += 1;
                    continue;
                }
                Err(e) => {
                    failed.push(format!("{}: {e}", chunk[0]));
                    break Vec::new();
                }
            }
        };
        if let Err(e) = validate_against_batch(&parsed, chunk) {
            failed.push(e);
            continue;
        }
        all_suggestions.extend(parsed);
    }

    let storage = state.storage.lock().unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    let rows: Vec<(
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
    )> = all_suggestions
        .iter()
        .map(|s| {
            (
                s.note_id.clone(),
                s.category.clone(),
                s.new_category_proposal.clone(),
                s.summary.clone(),
                if s.keywords.is_empty() {
                    None
                } else {
                    Some(serde_json::to_string(&s.keywords).unwrap_or_else(|_| "[]".into()))
                },
                now.clone(),
            )
        })
        .collect();
    suggestions_db::insert_batch(storage.conn(), &rows)?;
    for s in &all_suggestions {
        notes_db::set_ai_status(storage.conn(), &s.note_id, "suggested")?;
    }
    if let Some(summary) = &daily_summary {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        suggestions_db::upsert_daily_summary(storage.conn(), &today, summary)?;
    }
    drop(storage);

    let mut cfg = state.config.lock().unwrap();
    cfg.last_org_date = Some(chrono::Local::now().format("%Y-%m-%d").to_string());
    let root = state.data_dir.lock().unwrap().clone();
    crate::config::save(&root, &cfg).map_err(AppError::new)?;

    Ok(OrgRunResult {
        processed: batch.len(),
        suggested: all_suggestions.len(),
        failed,
    })
}

#[tauri::command]
pub async fn run_ai_org(app: AppHandle) -> AppResult<OrgRunResult> {
    run_ai_org_inner(&app).await
}

fn resolve_category_id(conn: &rusqlite::Connection, name: &str) -> AppResult<i64> {
    if let Some(id) = categories_db::id_by_name(conn, name)? {
        return Ok(id);
    }
    if categories_db::active_count(conn)? >= categories_db::MAX_ACTIVE_CATEGORIES {
        return Ok(categories_db::fallback_id(conn)?);
    }
    Ok(categories_db::create(conn, name, "ai")?.id)
}

#[tauri::command]
pub fn accept_suggestion(
    state: State<AppState>,
    suggestion_id: i64,
    category_name: Option<String>,
    summary: Option<String>,
    keywords: Option<String>,
) -> AppResult<()> {
    let storage = state.storage.lock().unwrap();
    let conn = storage.conn();
    let sug =
        suggestions_db::get(conn, suggestion_id)?.ok_or_else(|| AppError::new("建议不存在"))?;
    let cat_name = category_name.unwrap_or_else(|| {
        sug.new_category_proposal
            .clone()
            .or(sug.ai_category.clone())
            .unwrap_or_else(|| "其他".to_string())
    });
    let category_id = resolve_category_id(conn, &cat_name)?;
    notes_db::update_ai_meta(
        conn,
        &sug.note_id,
        Some(category_id),
        summary.as_deref(),
        keywords.as_deref(),
    )?;
    suggestions_db::set_status(conn, suggestion_id, "accepted")?;
    Ok(())
}

#[tauri::command]
pub fn skip_suggestion(state: State<AppState>, suggestion_id: i64) -> AppResult<()> {
    let storage = state.storage.lock().unwrap();
    let conn = storage.conn();
    let sug =
        suggestions_db::get(conn, suggestion_id)?.ok_or_else(|| AppError::new("建议不存在"))?;
    notes_db::set_ai_status(conn, &sug.note_id, "skipped")?;
    suggestions_db::set_status(conn, suggestion_id, "skipped")?;
    Ok(())
}

#[tauri::command]
pub fn accept_all(state: State<AppState>, date: String) -> AppResult<usize> {
    let storage = state.storage.lock().unwrap();
    let conn = storage.conn();
    let list = suggestions_db::list_by_date(conn, &date)?;
    let mut accepted = 0usize;
    for s in list.iter().filter(|s| s.status == "suggested") {
        let cat = s
            .new_category_proposal
            .clone()
            .or(s.ai_category.clone())
            .unwrap_or_else(|| "其他".to_string());
        let category_id = resolve_category_id(conn, &cat)?;
        notes_db::update_ai_meta(
            conn,
            &s.note_id,
            Some(category_id),
            s.summary.as_deref(),
            s.keywords.as_deref(),
        )?;
        suggestions_db::set_status(conn, s.id, "accepted")?;
        accepted += 1;
    }
    Ok(accepted)
}

#[tauri::command]
pub fn list_suggestions(
    state: State<AppState>,
    date: String,
) -> AppResult<Vec<suggestions_db::Suggestion>> {
    let storage = state.storage.lock().unwrap();
    Ok(suggestions_db::list_by_date(storage.conn(), &date)?)
}

#[tauri::command]
pub fn pending_suggestion_count(state: State<AppState>) -> AppResult<i64> {
    let storage = state.storage.lock().unwrap();
    let count = storage.conn().query_row(
        "SELECT COUNT(*) FROM suggestions WHERE status = 'suggested'",
        [],
        |r| r.get(0),
    )?;
    Ok(count)
}
