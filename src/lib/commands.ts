// Design Ref: §4.3 — Tauri invoke 래퍼. 컴포넌트에서 직접 invoke 금지 (CLAUDE.md 규칙).
// Plan SC-7: checkEnvironment가 5개 EnvItem 반환하는 EnvStatus를 리턴.

import { Channel, invoke } from "@tauri-apps/api/core";
import type { DiskCheck, EnvStatus, InstallProgress } from "./types";

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

// ─── 후속 피처 placeholder (Phase 1 scope 외, 시그니처 유지) ─────────────────

export async function downloadYoutube(
  url: string,
  outDir: string,
): Promise<string> {
  return invoke<string>("download_youtube", { url, outDir });
}

export async function extractAudio(
  videoPath: string,
  outDir: string,
): Promise<string> {
  return invoke<string>("extract_audio", { videoPath, outDir });
}

export async function separateAudio(
  filePath: string,
  model: string,
  outDir: string,
): Promise<string> {
  return invoke<string>("separate_audio", { filePath, model, outDir });
}

export async function exportMix(
  outputPath: string,
  format: string,
): Promise<string> {
  return invoke<string>("export_mix", { outputPath, format });
}
