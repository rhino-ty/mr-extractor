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

// ─── Navigation Payload (queue-page Phase 1, Design §3.2) ────────────────────
// FR-19: model literal "htdemucs_ft" — Phase 3 시점엔 고정값.
// v1.1 ModelSelector 도입 시 ModelId union으로 확장 (fix O).

export type NavigatePayload =
  | { kind: "queue" }
  | { kind: "process"; ids: string[]; model: "htdemucs_ft" }
  | { kind: "player"; itemId: string }
  | { kind: "history" }
  | { kind: "settings" }
  | { kind: "setup" };
