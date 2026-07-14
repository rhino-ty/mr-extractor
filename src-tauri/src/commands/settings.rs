// Design Ref: SETTINGS.md — 저장 공간 현황 + 임시 파일 정리.
// 환경 상태 패널은 setup::check_environment 재사용 (SETTINGS.md Current Rules).
// 설정 값 자체(알림/기본 포맷)는 프론트 Tauri Store(settings.json)가 관리.

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::commands::common;

// ─── Payloads ────────────────────────────────────────────────────────────────

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageStats {
    /// 앱 데이터 전체 (%APPDATA%/com.rhinoty.mr-extractor/) MB
    pub app_data_mb: u64,
    /// 임시 파일 (queue-tmp/) MB — 다운로드 wav + 분리 스템
    pub queue_tmp_mb: u64,
    /// AI 모델 캐시 (torch-cache/) MB
    pub model_cache_mb: u64,
    /// 내보내기 출력 폴더 (~/Desktop/MR Extractor/)
    pub output_dir: String,
    /// 출력 폴더 사용량 MB (폴더 없으면 0)
    pub output_mb: u64,
}

// ─── Commands ────────────────────────────────────────────────────────────────

/// 저장 공간 현황 — common::dir_size 실측 (SETTINGS.md "용량 계산: Rust 비동기 스캔").
#[tauri::command]
pub async fn storage_stats(app: AppHandle) -> Result<StorageStats, String> {
    let app_data = common::app_data_dir(&app)?;
    let queue_tmp = common::queue_tmp_dir(&app)?;
    let torch_cache = common::torch_cache_path(&app)?;
    let output_dir = app
        .path()
        .desktop_dir()
        .map(|d| d.join("MR Extractor"))
        .map_err(|_| "바탕화면 경로를 찾을 수 없어요.".to_string())?;

    // dir_size는 blocking 재귀 스캔 — spawn_blocking으로 UI 스레드 영향 차단
    let (app_data_mb, queue_tmp_mb, model_cache_mb, output_mb) = {
        let (a, q, t, o) = (
            app_data.clone(),
            queue_tmp.clone(),
            torch_cache.clone(),
            output_dir.clone(),
        );
        tokio::task::spawn_blocking(move || {
            (
                common::dir_size(&a) / 1024 / 1024,
                common::dir_size(&q) / 1024 / 1024,
                common::dir_size(&t) / 1024 / 1024,
                common::dir_size(&o) / 1024 / 1024,
            )
        })
        .await
        .map_err(|e| e.to_string())?
    };

    Ok(StorageStats {
        app_data_mb,
        queue_tmp_mb,
        model_cache_mb,
        output_dir: output_dir.to_string_lossy().to_string(),
        output_mb,
    })
}

/// 임시 파일 전체 정리 (queue-tmp/). 분리 스템도 포함되므로 프론트에서 경고 후 호출.
/// 처리 중에는 프론트가 버튼 비활성 (isProcessing 가드).
#[tauri::command]
pub async fn clear_queue_tmp(app: AppHandle) -> Result<u64, String> {
    let tmp = common::queue_tmp_dir(&app)?;
    let freed = tokio::task::spawn_blocking({
        let tmp = tmp.clone();
        move || common::dir_size(&tmp) / 1024 / 1024
    })
    .await
    .map_err(|e| e.to_string())?;

    if tmp.exists() {
        tokio::fs::remove_dir_all(&tmp)
            .await
            .map_err(|e| format!("임시 파일을 정리할 수 없어요: {}", e))?;
    }
    common::dev_log(&app, &format!("settings:clear_queue_tmp freed {}MB", freed));
    Ok(freed)
}
