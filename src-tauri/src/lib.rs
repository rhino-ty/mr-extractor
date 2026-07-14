// Design Ref: §2.3 -- Builder + 플러그인 등록 + 커맨드 등록

mod commands;
mod history;

use commands::{setup, youtube, separate, video, export, model, settings};
use commands::setup::InstallHandle;
use commands::queue::{self, QueueHandle};  // queue-page Phase 2 (Design §11.3) + Phase 3 cancel
use history::HistoryLock;  // history-page — history.json read-modify-write 직렬화

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
        // State (Design §6.3 — install_dependencies <-> cancel_install 공유)
        .manage(InstallHandle::default())
        // queue-page Phase 2 — extract_audio 등이 PID hook 등록. Phase 3에서 cancel_queue_item 사용.
        .manage(QueueHandle::default())
        // history-page — history.json 동시 쓰기 직렬화
        .manage(HistoryLock::default())
        // Iterate 1 (I-1) — Plan NFR Reliability / SC-9: startup orphan tmp cleanup (24h grace).
        // 백그라운드 task로 실행 — 실패가 앱 시작을 막지 않음 (silent).
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                queue::cleanup_orphan_tmp_files(handle).await;
            });
            Ok(())
        })
        // Commands (Design §4.1 Command List)
        .invoke_handler(tauri::generate_handler![
            setup::check_environment,
            setup::install_dependencies,
            setup::check_internet,
            setup::check_disk_space,
            setup::cancel_install,
            #[cfg(debug_assertions)]
            setup::read_setup_log,
            #[cfg(debug_assertions)]
            setup::clear_setup_log,
            #[cfg(debug_assertions)]
            setup::setup_log_path,
            youtube::download_youtube,             // queue-page Phase 3
            youtube::fetch_youtube_metadata,        // queue-page Phase 3
            queue::cancel_queue_item,               // queue-page Phase 3
            separate::separate_audio,
            video::extract_audio,
            video::fetch_video_metadata,            // queue-page Phase 2
            export::export_mix,
            history::history_list,                  // history-page
            history::history_upsert,
            history::history_set_export,
            history::history_remove,
            model::list_models,                     // model-selector v1.1
            model::download_model_by_name,
            settings::storage_stats,                // settings-page v1.2
            settings::clear_queue_tmp,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
