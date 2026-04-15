// Design Ref: §4 -- Tauri invoke 래퍼. 컴포넌트에서 직접 invoke 금지.
// 실제 로직은 이후 피처에서 구현. 현재는 빈 구조만.

import { invoke } from "@tauri-apps/api/core";

export async function checkEnvironment(): Promise<string> {
  return invoke<string>("check_environment");
}

export async function installDependencies(): Promise<string> {
  return invoke<string>("install_dependencies");
}

export async function downloadYoutube(
  url: string,
  outDir: string
): Promise<string> {
  return invoke<string>("download_youtube", { url, outDir });
}

export async function extractAudio(
  videoPath: string,
  outDir: string
): Promise<string> {
  return invoke<string>("extract_audio", { videoPath, outDir });
}

export async function separateAudio(
  filePath: string,
  model: string,
  outDir: string
): Promise<string> {
  return invoke<string>("separate_audio", { filePath, model, outDir });
}

export async function exportMix(
  outputPath: string,
  format: string
): Promise<string> {
  return invoke<string>("export_mix", { outputPath, format });
}
