mod ai;
mod commands;
mod config;
mod errors;
mod io_atomic;
mod notes_fs;
mod paths;
mod state;
mod storage;

use state::AppState;
use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let data_dir = config::default_data_dir();
    let cfg = config::load(&data_dir).expect("配置初始化失败");
    std::fs::create_dir_all(&cfg.data_dir).expect("数据目录创建失败");
    let storage =
        storage::Storage::open(&cfg.data_dir.join("quicknote.db")).expect("数据库初始化失败");
    let ai = ai::AiClient::new(
        cfg.ai.base_url.clone(),
        cfg.ai.model.clone(),
        cfg.ai.api_key.clone(),
    );
    let state = AppState::new(cfg, storage, ai);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .setup(|app| {
            spawn_scheduler(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::save_note,
            commands::list_notes,
            commands::get_note,
            commands::rebuild_index,
            commands::list_categories,
            commands::run_ai_org,
            commands::accept_suggestion,
            commands::skip_suggestion,
            commands::accept_all,
            commands::list_suggestions,
            commands::pending_suggestion_count,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn spawn_scheduler(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            let due = {
                let state = app.state::<AppState>();
                let cfg = state.config.lock().unwrap();
                ai::schedule::is_due(
                    chrono::Local::now(),
                    &cfg.org_time,
                    cfg.last_org_date.as_deref(),
                ) && cfg.auto_org_enabled
            };
            if due {
                let _ = app.emit("ai-org-started", ());
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    match commands::run_ai_org_inner(&app).await {
                        Ok(result) => {
                            let _ = app.emit("ai-org-completed", result);
                        }
                        Err(e) => {
                            let _ = app.emit("ai-org-error", e.error);
                        }
                    }
                });
            }
        }
    });
}
