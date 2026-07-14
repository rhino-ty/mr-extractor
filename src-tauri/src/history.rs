// Design Ref: HISTORY.md — 처리 히스토리 JSON 관리.
// 저장: %APPDATA%/com.rhinoty.mr-extractor/history.json (HISTORY.md Current Rules)
// CLAUDE.md 구조: src-tauri/src/history.rs (commands/ 밖 — 히스토리 도메인 모듈)
//
// 섹션:
//   § 1. Data Model     — HistoryEntry (HISTORY.md JSON 구조 준수) + View (존재 여부 계산)
//   § 2. Storage        — 파일 read/write (HistoryLock Mutex로 동시 쓰기 직렬화)
//   § 3. Commands       — history_list / history_upsert / history_set_export / history_remove

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tokio::sync::Mutex;

use crate::commands::common;

// ═══════════════════════════════════════════════════════════════════════════════
// § 1. Data Model
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryStems {
    pub vocals: String,
    pub drums: String,
    pub bass: String,
    pub other: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryFiles {
    pub wav: Option<String>,
    pub mp3: Option<String>,
    pub stems: Option<HistoryStems>,
}

/// HISTORY.md JSON 구조. id는 QueueItem.id(UUID) 재사용 — 같은 항목 재처리 시 갱신.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub date: String,
    pub source_type: String,
    pub source: String,
    pub title: String,
    pub model: String,
    pub out_dir: String,
    pub files: HistoryFiles,
    pub status: String,
    pub error_msg: Option<String>,
}

/// list 응답 전용 — 파일 존재 여부를 Rust에서 일괄 계산 (HISTORY.md 뱃지 3종).
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntryView {
    #[serde(flatten)]
    pub entry: HistoryEntry,
    /// 4 stems 모두 존재 → [🎚 스템 있음] 뱃지 + PlayerPage 재생 가능
    pub stems_exist: bool,
    /// instrumental wav/mp3 중 하나라도 존재 → [🎵 반주만]
    pub inst_exists: bool,
}

#[derive(Serialize, Deserialize, Default)]
struct HistoryFileRoot {
    entries: Vec<HistoryEntry>,
}

/// read-modify-write 직렬화용 State.
#[derive(Default)]
pub struct HistoryLock(pub Mutex<()>);

// ═══════════════════════════════════════════════════════════════════════════════
// § 2. Storage
// ═══════════════════════════════════════════════════════════════════════════════

fn history_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(common::app_data_dir(app)?.join("history.json"))
}

/// 읽기 실패 구분 (code-analyzer Critical fix):
///   - NotFound → 빈 히스토리 (정상)
///   - 그 외 IO 에러 → Err 전파 (쓰기 경로가 기존 데이터를 빈 목록으로 덮어쓰지 않도록)
///   - JSON 파손 → .corrupt로 백업 후 빈 히스토리로 재시작 (silent 소실 방지)
async fn read_root(app: &AppHandle) -> Result<HistoryFileRoot, String> {
    let path = history_path(app)?;
    match tokio::fs::read_to_string(&path).await {
        Ok(text) => match serde_json::from_str(&text) {
            Ok(root) => Ok(root),
            Err(e) => {
                common::dev_log(
                    app,
                    &format!("history:read_root: JSON 파손 — .corrupt 백업 후 초기화 ({})", e),
                );
                let backup = path.with_extension("json.corrupt");
                let _ = tokio::fs::rename(&path, &backup).await;
                Ok(HistoryFileRoot::default())
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(HistoryFileRoot::default()),
        Err(e) => Err(format!("히스토리를 읽을 수 없어요: {}", e)),
    }
}

/// 원자적 쓰기: tmp에 기록 후 rename (NTFS rename은 원자적 — 중단 시 파손 방지).
async fn write_root(app: &AppHandle, root: &HistoryFileRoot) -> Result<(), String> {
    let path = history_path(app)?;
    if let Some(dir) = path.parent() {
        let _ = tokio::fs::create_dir_all(dir).await;
    }
    let text = serde_json::to_string_pretty(root).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    tokio::fs::write(&tmp, text)
        .await
        .map_err(|e| format!("히스토리를 저장할 수 없어요: {}", e))?;
    tokio::fs::rename(&tmp, &path)
        .await
        .map_err(|e| format!("히스토리를 저장할 수 없어요: {}", e))
}

fn stems_exist(files: &HistoryFiles) -> bool {
    files.stems.as_ref().is_some_and(|s| {
        [&s.vocals, &s.drums, &s.bass, &s.other]
            .iter()
            .all(|p| Path::new(p).exists())
    })
}

fn inst_exists(files: &HistoryFiles) -> bool {
    files
        .wav
        .as_ref()
        .is_some_and(|p| Path::new(p).exists())
        || files
            .mp3
            .as_ref()
            .is_some_and(|p| Path::new(p).exists())
}

// ═══════════════════════════════════════════════════════════════════════════════
// § 3. Commands
// ═══════════════════════════════════════════════════════════════════════════════

/// 최신순 정렬 + 파일 존재 여부 계산 결과 반환.
/// 쓰기와 동시 접근 시 일관성 보장을 위해 read도 lock (code-analyzer fix).
#[tauri::command]
pub async fn history_list(
    app: AppHandle,
    lock: State<'_, HistoryLock>,
) -> Result<Vec<HistoryEntryView>, String> {
    let _g = lock.0.lock().await;
    let root = read_root(&app).await?;
    let mut views: Vec<HistoryEntryView> = root
        .entries
        .into_iter()
        .map(|entry| {
            let se = stems_exist(&entry.files);
            let ie = inst_exists(&entry.files);
            HistoryEntryView {
                entry,
                stems_exist: se,
                inst_exists: ie,
            }
        })
        .collect();
    views.sort_by(|a, b| b.entry.date.cmp(&a.entry.date));
    Ok(views)
}

/// 분리 완료/실패 시 process.ts가 호출. 같은 id 존재 시 교체 (재처리).
#[tauri::command]
pub async fn history_upsert(
    app: AppHandle,
    entry: HistoryEntry,
    lock: State<'_, HistoryLock>,
) -> Result<(), String> {
    let _g = lock.0.lock().await;
    let mut root = read_root(&app).await?;
    root.entries.retain(|e| e.id != entry.id);
    root.entries.push(entry);
    write_root(&app, &root).await
}

/// 내보내기 완료 시 wav/mp3 경로 기록 (ExportPanel 호출). 항목 없으면 무시(Ok).
#[tauri::command]
pub async fn history_set_export(
    app: AppHandle,
    id: String,
    format: String,
    path: String,
    lock: State<'_, HistoryLock>,
) -> Result<(), String> {
    let _g = lock.0.lock().await;
    let mut root = read_root(&app).await?;
    if let Some(entry) = root.entries.iter_mut().find(|e| e.id == id) {
        match format.as_str() {
            "mp3" => entry.files.mp3 = Some(path),
            // flac도 무손실 산출물 슬롯(wav)에 기록 — HISTORY.md 스키마 유지
            _ => entry.files.wav = Some(path),
        }
        write_root(&app, &root).await?;
    }
    Ok(())
}

/// 기록 삭제. delete_files=true면 산출물도 삭제
/// (스템 디렉토리 {queue-tmp}/{id}/ + 내보내기 wav/mp3 — HISTORY.md 삭제 다이얼로그).
#[tauri::command]
pub async fn history_remove(
    app: AppHandle,
    ids: Vec<String>,
    delete_files: bool,
    lock: State<'_, HistoryLock>,
) -> Result<(), String> {
    let _g = lock.0.lock().await;
    let mut root = read_root(&app).await?;

    if delete_files {
        let tmp_dir = common::queue_tmp_dir(&app)?;
        for entry in root.entries.iter().filter(|e| ids.contains(&e.id)) {
            // 스템: {queue-tmp}/{id}/ 트리 통째로
            let _ = tokio::fs::remove_dir_all(tmp_dir.join(&entry.id)).await;
            let _ = tokio::fs::remove_file(tmp_dir.join(format!("{}.wav", entry.id))).await;
            // 내보내기 산출물
            if let Some(p) = &entry.files.wav {
                let _ = tokio::fs::remove_file(p).await;
            }
            if let Some(p) = &entry.files.mp3 {
                let _ = tokio::fs::remove_file(p).await;
            }
        }
    }

    root.entries.retain(|e| !ids.contains(&e.id));
    write_root(&app, &root).await?;
    common::dev_log(
        &app,
        &format!("history:remove {} entries (files={})", ids.len(), delete_files),
    );
    Ok(())
}
