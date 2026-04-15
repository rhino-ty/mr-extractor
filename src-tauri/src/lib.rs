// Design Ref: §2.3 -- Builder + 플러그인 등록 + 커맨드 등록

mod commands;

use commands::{setup, youtube, separate, video, export};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Plugins (Design §2.3 Dependencies)
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_global_shortcut::Builder::default().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_process::init())
        // Commands
        .invoke_handler(tauri::generate_handler![
            setup::check_environment,
            setup::install_dependencies,
            youtube::download_youtube,
            separate::separate_audio,
            video::extract_audio,
            export::export_mix,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
