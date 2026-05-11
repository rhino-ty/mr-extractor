// Design Ref: §3.1 / §4.2 / §11.3 — queue-page Phase 2/3 공유 State + cancel command
// Phase 2: video.rs::extract_audio가 PID hook으로 register/take 사용 (fix III)
// Phase 3: cancel_queue_item Tauri command 본구현 (fix IV)
// Iterate 1 (I-1): startup orphan tmp cleanup (Plan NFR Reliability / SC-9, 24h grace)
//
// Plan FR-18 — 처리 중 항목의 [✕] 클릭 → tree kill + tmp cleanup.
// 의존: std::sync::Mutex + std::collections::HashMap + AppHandle/State.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use tauri::{AppHandle, State};

use crate::commands::common;

/// Orphan tmp 파일 cleanup grace period. Plan NFR Reliability: 24h.
const ORPHAN_GRACE: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Default)]
pub struct QueueHandle(pub Mutex<HashMap<String, u32>>);

impl QueueHandle {
    /// 처리 시작 시 child process PID 등록. ID는 QueueItem.id (UUID v4).
    pub fn register(&self, id: String, pid: u32) {
        if let Ok(mut g) = self.0.lock() {
            g.insert(id, pid);
        }
    }

    /// 처리 종료/취소 시 PID 슬롯 비움. 멱등성: 없는 id 호출도 None 반환.
    pub fn take(&self, id: &str) -> Option<u32> {
        self.0.lock().ok().and_then(|mut g| g.remove(id))
    }
}

/// Plan FR-18 / Design §4.2 — 처리 중 항목 cancel.
/// 1. QueueHandle.take(id)로 PID 조회 (없으면 멱등 Ok)
/// 2. common::kill_process_tree로 트리 kill (Windows taskkill /F /T /PID)
/// 3. queue_tmp_dir/{id}.* 파일 best-effort cleanup (yt-dlp ext 미상 → prefix 매칭)
#[tauri::command]
pub async fn cancel_queue_item(
    app: AppHandle,
    item_id: String,
    handle: State<'_, QueueHandle>,
) -> Result<(), String> {
    let Some(pid) = handle.take(&item_id) else {
        return Ok(()); // 멱등성 — 이미 종료/완료됨
    };
    common::dev_log(
        &app,
        &format!("queue:cancel_queue_item({}): kill PID {}", item_id, pid),
    );
    common::kill_process_tree(pid)?;

    // tmp 파일 cleanup (best-effort)
    let tmp_dir = common::queue_tmp_dir(&app)?;
    if let Ok(mut entries) = tokio::fs::read_dir(&tmp_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with(&item_id) {
                    let _ = tokio::fs::remove_file(entry.path()).await;
                }
            }
        }
    }
    Ok(())
}

/// Iterate 1 (I-1) / Plan NFR Reliability / SC-9 — startup orphan tmp cleanup.
/// `%APPDATA%/com.rhinoty.mr-extractor/queue-tmp/` 안에서 modified time이
/// 현재 시각 - 24h 이전인 파일을 silent 삭제. 에러는 dev_log로만 기록.
///
/// 호출처: `lib.rs::run()`의 `setup` 클로저 — `tokio::spawn`으로 백그라운드 실행.
/// 디렉토리가 없으면 (첫 실행 등) 조용히 패스.
pub async fn cleanup_orphan_tmp_files(app: AppHandle) {
    let Ok(tmp_dir) = common::queue_tmp_dir(&app) else {
        return;
    };

    let mut entries = match tokio::fs::read_dir(&tmp_dir).await {
        Ok(it) => it,
        Err(_) => return, // 디렉토리 없음 — 정상 (queue-tmp는 lazy 생성)
    };

    let now = SystemTime::now();
    let mut removed = 0_u32;
    let mut scanned = 0_u32;

    while let Ok(Some(entry)) = entries.next_entry().await {
        scanned += 1;
        let path = entry.path();
        let Ok(meta) = entry.metadata().await else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        let Ok(modified) = meta.modified() else {
            continue;
        };
        let Ok(age) = now.duration_since(modified) else {
            continue; // future timestamp (clock skew) — skip
        };
        if age >= ORPHAN_GRACE {
            if tokio::fs::remove_file(&path).await.is_ok() {
                removed += 1;
            }
        }
    }

    common::dev_log(
        &app,
        &format!(
            "queue:cleanup_orphan_tmp_files: scanned={}, removed={} (>=24h)",
            scanned, removed
        ),
    );
}
