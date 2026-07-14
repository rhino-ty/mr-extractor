// Design Ref: §3.2 — Rust struct와 필드명 일치 (camelCase ↔ snake_case serde rename)

export type PageName =
  | "setup"
  | "queue"
  | "process"
  | "player"
  | "history"
  | "settings";

// ─── Environment Status (Rust: setup::EnvStatus) ─────────────────────────────

export type EnvItemStatus = "ready" | "missing" | "installing" | "error";

export interface EnvItem {
  label: string;
  status: EnvItemStatus;
  version: string | null;
}

export interface EnvStatus {
  items: EnvItem[];
  allReady: boolean;
  installSizeEstimateMb: number;
  sizeProbeSucceeded: boolean;
}

// ─── Install Progress (Rust: setup::InstallProgress via Channel) ─────────────

export type InstallPhase =
  | "extract_python"
  | "create_venv"
  | "install_torch"
  | "install_demucs"
  | "download_model";

export interface InstallProgress {
  step: string;
  percent: number;
  phase: InstallPhase;
  currentSizeMb: number | null;
  estimatedFinalMb: number;
}

// ─── SetupPage 상태 머신 (Design §5.1, 6-state tagged union) ─────────────────
// Plan SC-2: 2회차 이후 detecting → ready 바로 전환.

export interface DiskBreakdown {
  install: number;
  model: number;
  staging: number;
  headroom: number;
  total: number;
}

// ─── Disk Check (Rust: setup::DiskCheck) ─────────────────────────────────────
// Phase 3 신규. Plan FR-11 / SC-9 / SC-12.

export interface DiskCheck {
  fits: boolean;
  freeMb: number;
  breakdown: DiskBreakdown;
  sizeProbeSucceeded: boolean;
}

export type SetupPageState =
  | { kind: "detecting" }
  | {
      kind: "prompt-install";
      items: EnvItem[];
      breakdown: DiskBreakdown;
      freeMb: number;
      sizeProbeOk: boolean;
    }
  | { kind: "installing"; progress: InstallProgress; items: EnvItem[] }
  | { kind: "ready"; items: EnvItem[]; sizeMb: number }
  | { kind: "error"; items: EnvItem[]; message: string; detail: string }
  | { kind: "no-internet" }
  | { kind: "disk-full"; required: DiskBreakdown; current: number };

// ─── Queue (queue-page Phase 1) ──────────────────────────────────────────────
// Design Ref: §3.2 — HISTORY.md `source_type` 명명과 일치

export type QueueSourceType = "youtube" | "file" | "video";

export type QueueItemStatus =
  | "pending"
  | "fetching-metadata"
  | "downloading"
  | "extracting"
  | "ready-to-separate"
  | "in-progress"
  | "done"
  | "error";

export interface QueueItem {
  id: string;
  sourceType: QueueSourceType;
  source: string;
  label: string;
  durationSec?: number;
  tmpPath?: string;
  progress: number;
  step?: string;
  status: QueueItemStatus;
  errorDetail?: string;
  addedAt: string;
  // process-page FR-14 — 분리 성공 시 4 stems 절대경로. PlayerPage가 읽어 로드.
  // (FR-16의 progress?: {percent, step}는 기존 progress + step 필드가 이미 충족)
  outputs?: StemOutputs;
}

// ─── Separation (process-page Phase 2, Rust: separate.rs) ───────────────────

export interface StemOutputs {
  vocals: string;
  drums: string;
  bass: string;
  other: string;
}

export interface SeparationProgress {
  itemId: string;
  step: string;
  percent: number;
}

export interface SeparationResult {
  itemId: string;
  vocals: string;
  drums: string;
  bass: string;
  other: string;
}

// ─── Export (player-page, Rust: export.rs) ──────────────────────────────────

export type ExportFormat = "wav" | "mp3" | "flac";

export interface StemExportConfig {
  path: string;
  volume: number; // 0.0 ~ 1.0
  muted: boolean;
}

export interface ExportProgress {
  step: string;
  percent: number;
}

// ─── History (history-page, Rust: history.rs — HISTORY.md JSON 구조) ─────────

export interface HistoryStems {
  vocals: string;
  drums: string;
  bass: string;
  other: string;
}

export interface HistoryFiles {
  wav: string | null;
  mp3: string | null;
  stems: HistoryStems | null;
}

export type HistoryStatus = "done" | "error";

export interface HistoryEntry {
  id: string;
  date: string;
  sourceType: QueueSourceType;
  source: string;
  title: string;
  model: string;
  outDir: string;
  files: HistoryFiles;
  status: HistoryStatus;
  errorMsg: string | null;
}

/// history_list 응답 — 파일 존재 여부는 Rust가 일괄 계산 (뱃지 3종).
export interface HistoryEntryView extends HistoryEntry {
  stemsExist: boolean;
  instExists: boolean;
}

// Design Ref: §3.1 — Rust Channel payload (Phase 2/3에서 호출)

export interface VideoMetadata {
  itemId: string;
  durationSec: number;
}

export interface YoutubeMetadata {
  itemId: string;
  title: string;
  durationSec: number;
}

export interface DownloadProgress {
  itemId: string;
  step: string;
  percent: number;
}

export interface ExtractProgress {
  itemId: string;
  percent: number;
}

// ─── Toast (queue-page Phase 1, Plan §10.2 자체 구현) ────────────────────────

export type ToastKind = "info" | "success" | "warn" | "error";

export interface Toast {
  id: string;
  kind: ToastKind;
  message: string;
  durationMs: number;
}

// ─── Model (model-selector v1.1, Rust: model.rs) ────────────────────────────

export type ModelId = "htdemucs" | "htdemucs_ft" | "htdemucs_6s";

export interface ModelInfo {
  id: ModelId;
  installed: boolean;
}

export interface ModelDownloadProgress {
  model: string;
  step: string;
  percent: number;
}

// ─── Navigation Payload (queue-page Phase 1, Design §3.2) ────────────────────
// v1.1 ModelSelector — model이 ModelId union으로 확장됨 (queue-page fix O 예정 반영).

export type NavigatePayload =
  | { kind: "queue" }
  | { kind: "process"; ids: string[]; model: ModelId }
  | { kind: "player"; itemId: string }
  | { kind: "history" }
  | { kind: "settings" }
  | { kind: "setup" };
