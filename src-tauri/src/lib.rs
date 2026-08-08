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
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, WindowEvent,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutEvent, ShortcutState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let data_dir = config::load_data_dir_pointer().unwrap_or_else(config::default_data_dir);
    let cfg = config::load(&data_dir).expect("配置初始化失败");
    std::fs::create_dir_all(&cfg.data_dir).expect("数据目录创建失败");
    let storage =
        storage::Storage::open(&cfg.data_dir.join("quicknote.db")).expect("数据库初始化失败");
    let state = AppState::new(cfg, storage);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .manage(state)
        .setup(setup)
        .invoke_handler(tauri::generate_handler![
            commands::save_note,
            commands::list_notes,
            commands::list_notes_by_category,
            commands::get_data_dir,
            commands::get_settings,
            commands::update_settings,
            commands::init_data_dir,
            commands::migrate_data_dir,
            commands::get_note,
            commands::update_note,
            commands::save_image,
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

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // 关闭按钮改为隐藏窗口，而不是销毁：保证托盘常驻后快捷键仍可呼出。
    for label in ["main", "capture"] {
        if let Some(w) = app.get_webview_window(label) {
            let w2 = w.clone();
            w.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = w2.hide();
                }
            });
        }
    }

    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "打开 QuickNote", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    TrayIconBuilder::with_id("main")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => app.exit(0),
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            _ => {}
        })
        .build(app)?;

    let capture = app
        .get_webview_window("capture")
        .ok_or("capture window missing")?;
    let hotkey = app.state::<AppState>().config.lock().unwrap().hotkey.clone();
    let shortcut: Shortcut = hotkey.parse().map_err(|e| format!("快捷键格式错误: {e}"))?;
    app.global_shortcut()
        .on_shortcut(shortcut, move |app, _shortcut, event: ShortcutEvent| {
        if event.state() == ShortcutState::Pressed {
            if capture.is_visible().unwrap_or(false) {
                let _ = capture.hide();
            } else {
                let _ = capture.show();
                let _ = capture.set_focus();
                let _ = app.emit("capture-focus", ());
            }
        }
    })?;

    spawn_scheduler(app.handle().clone());
    Ok(())
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
