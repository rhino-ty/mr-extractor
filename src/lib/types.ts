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
