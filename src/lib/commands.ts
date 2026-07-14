// Design Ref: §4.3 — Tauri invoke 래퍼. 컴포넌트에서 직접 invoke 금지 (CLAUDE.md 규칙).
// Plan SC-7: checkEnvironment가 5개 EnvItem 반환하는 EnvStatus를 리턴.

import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  DiskCheck,
  DownloadProgress,
  EnvStatus,
  ExportFormat,
  ExportProgress,
  ExtractProgress,
  InstallProgress,
  SeparationProgress,
  SeparationResult,
  StemExportConfig,
  VideoMetadata,
  YoutubeMetadata,
} from "./types";

export async function checkEnvironment(): Promise<EnvStatus> {
  return invoke<EnvStatus>("check_environment");
}

export async function checkInternet(): Promise<boolean> {
  return invoke<boolean>("check_internet");
}

// Phase 3 신규. Plan FR-11.
export async function checkDiskSpace(): Promise<DiskCheck> {
  return invoke<DiskCheck>("check_disk_space");
}

export async function installDependencies(
  onProgress: (p: InstallProgress) => void,
): Promise<void> {
  const channel = new Channel<InstallProgress>();
  channel.onmessage = onProgress;
  return invoke<void>("install_dependencies", { onProgress: channel });
}

export async function cancelInstall(): Promise<void> {
  return invoke<void>("cancel_install");
}

// ─── Dev 진단용 (debug 빌드에서만 의미 있는 데이터, release는 빈 문자열) ──────

export async function readSetupLog(): Promise<string> {
  return invoke<string>("read_setup_log");
}

export async function clearSetupLog(): Promise<void> {
  return invoke<void>("clear_setup_log");
}

export async function getSetupLogPath(): Promise<string> {
  return invoke<string>("setup_log_path");
}

// ─── queue-page Phase 2 (Design §4.3) ────────────────────────────────────────

/// Plan FR-17 — ffprobe 단일 호출. 성공 시 durationSec > 0. duration=0 → corrupt err.
export async function fetchVideoMetadata(
  itemId: string,
  path: string,
): Promise<VideoMetadata> {
  return invoke<VideoMetadata>("fetch_video_metadata", { itemId, path });
}

/// Plan FR-06 — ffmpeg 추출 + Channel 진행률.
/// fix #1 — durationSec를 frontend가 캐시 후 인자 전달 (Rust 측 ffprobe 재호출 회피).
export async function extractAudio(
  itemId: string,
  path: string,
  durationSec: number,
  onProgress: (p: ExtractProgress) => void,
): Promise<string> {
  const channel = new Channel<ExtractProgress>();
  channel.onmessage = onProgress;
  return invoke<string>("extract_audio", {
    itemId,
    path,
    durationSec,
    onProgress: channel,
  });
}

// ─── queue-page Phase 3 (Design §4.3) ────────────────────────────────────────

/// Plan FR-17 — yt-dlp --skip-download --print으로 사전 추출 (~3~5초). 실패 시 friendly err.
export async function fetchYoutubeMetadata(
  itemId: string,
  url: string,
): Promise<YoutubeMetadata> {
  return invoke<YoutubeMetadata>("fetch_youtube_metadata", { itemId, url });
}

/// Plan FR-07 / FR-15 / FR-16 — yt-dlp 다운로드. --output queue_tmp/{id}.%(ext)s.
/// 영상이면 호출자가 fetch_video_metadata + extractAudio 체이닝.
export async function downloadYoutube(
  itemId: string,
  url: string,
  onProgress: (p: DownloadProgress) => void,
): Promise<string> {
  const channel = new Channel<DownloadProgress>();
  channel.onmessage = onProgress;
  return invoke<string>("download_youtube", { itemId, url, onProgress: channel });
}

/// Plan FR-18 — 처리 중 항목 cancel. 멱등성: 등록 안 된 id 호출도 Ok.
export async function cancelQueueItem(itemId: string): Promise<void> {
  return invoke<void>("cancel_queue_item", { itemId });
}

// ─── process-page Phase 2 (Plan §2.1) ────────────────────────────────────────

/// Plan FR-02/03/05 — demucs 분리 + Channel 진행률. 출력 위치는 Rust가
/// `{queue-tmp}/{id}/`로 산출 (frontend가 경로를 알 필요 없음 — queue-page 일관).
export async function separateAudio(
  itemId: string,
  filePath: string,
  model: string,
  onProgress: (p: SeparationProgress) => void,
): Promise<SeparationResult> {
  const channel = new Channel<SeparationProgress>();
  channel.onmessage = onProgress;
  return invoke<SeparationResult>("separate_audio", {
    itemId,
    filePath,
    model,
    onProgress: channel,
  });
}

// ─── player-page 내보내기 (export.rs) ────────────────────────────────────────

/// 현재 믹서 볼륨/뮤트 + 키 조절을 반영한 믹스다운.
/// durationSec는 프론트 캐시 전달 (ffprobe 재호출 회피 — queue-page fix #1 패턴).
/// 반환: 생성된 출력 파일 절대경로 (~/Desktop/MR Extractor/).
export async function exportMix(
  title: string,
  stems: StemExportConfig[],
  format: ExportFormat,
  semitones: number,
  durationSec: number,
  onProgress: (p: ExportProgress) => void,
): Promise<string> {
  const channel = new Channel<ExportProgress>();
  channel.onmessage = onProgress;
  return invoke<string>("export_mix", {
    title,
    stems,
    format,
    semitones,
    durationSec,
    onProgress: channel,
  });
}
