use serde::Serialize;
use tauri::State;

use crate::errors::{AppError, AppResult};
use crate::notes_fs::{generate_id, read_note_file, save_note_file, save_pasted_image, title_from_markdown};
use crate::state::AppState;
use crate::storage::categories as categories_db;
use crate::storage::notes as notes_db;

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
    let root = state.data_dir.clone();
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
pub fn get_note(state: State<AppState>, id: String) -> AppResult<Option<String>> {
    let storage = state.storage.lock().unwrap();
    let exists = notes_db::get_note(storage.conn(), &id)?;
    drop(storage);
    if exists.is_none() {
        return Ok(None);
    }
    Ok(Some(read_note_file(&state.data_dir, &id).map_err(AppError::new)?))
}

#[tauri::command]
pub fn rebuild_index(state: State<AppState>) -> AppResult<(usize, usize)> {
    let storage = state.storage.lock().unwrap();
    crate::storage::rebuild::rebuild(storage.conn(), &state.data_dir)
}

#[tauri::command]
pub fn list_categories(state: State<AppState>) -> AppResult<Vec<categories_db::Category>> {
    let storage = state.storage.lock().unwrap();
    Ok(categories_db::list(storage.conn())?)
}
