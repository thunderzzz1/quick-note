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
        .invoke_handler(tauri::generate_handler![
            commands::save_note,
            commands::list_notes,
            commands::get_note,
            commands::rebuild_index,
            commands::list_categories,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
