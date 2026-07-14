<script lang="ts">
  // Design Ref: UI.md [💾 현재 믹스 내보내기] — 포맷 선택 + 진행률 + 완료 액션.
  // 현재 믹서 상태(mixerStore)와 키 조절을 그대로 반영해 export_mix 호출.
  // 완료 시 시스템 알림 (ROADMAP v1 필수 기능) + [📁 폴더 열기].

  import { get } from "svelte/store";
  import { slide } from "svelte/transition";
  import { open as shellOpen } from "@tauri-apps/plugin-shell";
  import {
    isPermissionGranted,
    requestPermission,
    sendNotification,
  } from "@tauri-apps/plugin-notification";
  import { exportMix } from "$lib/commands";
  import { pushToast } from "$lib/stores";
  import {
    STEM_ORDER,
    getDuration,
    loadedTrack,
    mixerStore,
  } from "$lib/audio";
  import type { ExportFormat, StemExportConfig, StemOutputs } from "$lib/types";

  interface Props {
    outputs: StemOutputs;
  }

  let { outputs }: Props = $props();

  const FORMATS: Array<{ value: ExportFormat; label: string; hint: string }> = [
    { value: "wav", label: "WAV", hint: "무손실 · 기본" },
    { value: "mp3", label: "MP3 320k", hint: "공유용 · 용량 작음" },
    { value: "flac", label: "FLAC", hint: "무손실 압축" },
  ];

  let format = $state<ExportFormat>("wav");
  let exporting = $state(false);
  let percent = $state(0);
  let resultPath = $state<string | null>(null);

  let mixer = $derived($mixerStore);
  let vocalsAudible = $derived(
    !mixer.stems.vocals.muted && mixer.stems.vocals.volume > 0,
  );

  async function notifyDone(body: string): Promise<void> {
    try {
      let granted = await isPermissionGranted();
      if (!granted) {
        granted = (await requestPermission()) === "granted";
      }
      if (granted) {
        sendNotification({ title: "내보내기 완료", body });
      }
    } catch {
      // 알림 실패는 치명적이지 않음 — 토스트가 이미 안내
    }
  }

  async function runExport(): Promise<void> {
    if (exporting) return;
    exporting = true;
    percent = 0;
    resultPath = null;

    const m = get(mixerStore);
    const stems: StemExportConfig[] = STEM_ORDER.map((name) => ({
      path: outputs[name],
      volume: m.stems[name].volume / 100,
      muted: m.stems[name].muted,
    }));
    const title = get(loadedTrack).label;

    try {
      const path = await exportMix(
        title,
        stems,
        format,
        m.semitones,
        Math.ceil(getDuration()),
        (p) => {
          percent = p.percent;
        },
      );
      resultPath = path;
      pushToast("내보내기가 완료됐어요.", "success");
      void notifyDone(path.split(/[\\/]/).pop() ?? path);
    } catch (err) {
      const msg = typeof err === "string" ? err : String(err);
      pushToast(msg, "error");
    } finally {
      exporting = false;
    }
  }

  async function openFolder(): Promise<void> {
    if (!resultPath) return;
    const dir = resultPath.slice(
      0,
      Math.max(resultPath.lastIndexOf("\\"), resultPath.lastIndexOf("/")),
    );
    try {
      await shellOpen(dir);
    } catch {
      pushToast("폴더를 열 수 없어요.", "error");
    }
  }
</script>

<div
  class="flex flex-col gap-3 rounded-xl border border-border bg-surface p-4"
  transition:slide={{ duration: 200 }}
>
  <div class="flex items-center justify-between">
    <span class="text-sm font-semibold text-text">
      <span aria-hidden="true">💾</span>
      믹스 내보내기
    </span>
    <span class="text-xs text-muted">
      {vocalsAudible ? "보컬 포함 믹스로 저장돼요" : "반주(MR)로 저장돼요"}
      {#if mixer.semitones !== 0}
        · 키 {mixer.semitones > 0 ? `+${mixer.semitones}` : mixer.semitones}반음 적용
      {/if}
    </span>
  </div>

  <div class="flex gap-2">
    {#each FORMATS as f (f.value)}
      <button
        type="button"
        class="flex-1 rounded-lg border px-3 py-2 text-sm transition-colors duration-200 {format ===
        f.value
          ? 'border-accent bg-accent/20 text-text'
          : 'border-border text-muted hover:border-accent/40 hover:text-text'}"
        title={f.hint}
        onclick={() => (format = f.value)}
        disabled={exporting}
      >
        {f.label}
        <span class="block text-[11px] text-muted">{f.hint}</span>
      </button>
    {/each}
  </div>

  {#if exporting}
    <div class="flex items-center gap-3">
      <div class="h-1.5 flex-1 overflow-hidden rounded-full bg-bg">
        <div
          class="h-full bg-accent transition-all duration-200"
          style="width: {percent}%"
        ></div>
      </div>
      <span class="w-10 text-right text-xs text-muted">{percent}%</span>
    </div>
  {:else if resultPath}
    <div class="flex items-center justify-between gap-3 rounded-lg bg-bg/50 px-3 py-2">
      <span class="truncate text-xs text-success" title={resultPath}>
        ✓ {resultPath}
      </span>
      <button
        type="button"
        class="shrink-0 rounded-md border border-border px-2 py-1 text-xs text-muted transition-colors duration-200 hover:border-accent/40 hover:text-text"
        onclick={() => void openFolder()}
      >
        📁 폴더 열기
      </button>
    </div>
  {/if}

  <button
    type="button"
    class="rounded-lg bg-accent px-4 py-2 text-sm font-medium text-white transition-colors duration-200 hover:bg-accent/80 disabled:cursor-not-allowed disabled:opacity-50"
    onclick={() => void runExport()}
    disabled={exporting}
  >
    {exporting ? "내보내는 중..." : "💾 현재 믹스 내보내기"}
  </button>
</div>
