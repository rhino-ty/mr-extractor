<script lang="ts">
  // Design Ref: UI.md PlayerPage 레이아웃 — wavesurfer.js 파형 + 재생 위치 + A-B 구간.
  // 오디오 재생은 audio.ts(Tone.Transport)가 담당 — wavesurfer는 시각화 전용.
  // 사전 계산된 peaks + duration으로 렌더 (오디오 중복 디코딩 방지),
  // 재생 커서는 rAF 오버레이로 직접 그림 (media 없는 wavesurfer는 진행 렌더 불가).

  import { onDestroy, onMount } from "svelte";
  import WaveSurfer from "wavesurfer.js";
  import RegionsPlugin, {
    type Region,
  } from "wavesurfer.js/dist/plugins/regions.js";
  import {
    checkEnded,
    clearLoop,
    getDuration,
    getPeaks,
    getPosition,
    seek,
    setLoop,
  } from "$lib/audio";

  let container: HTMLDivElement;
  let ws: WaveSurfer | null = null;
  let regions: RegionsPlugin | null = null;
  let loopRegion: Region | null = null;
  let rafId = 0;

  let positionSec = $state(0);
  let hasLoop = $state(false);

  let duration = getDuration();

  // wavesurfer v7은 media 기반이라 peaks-only 렌더에도 url이 필요 → 무음 1샘플 wav
  function silentWavUri(): string {
    // RIFF/WAVE 헤더 44바이트 + 무음 samples 4바이트 (PCM 16bit mono 44100Hz)
    const b = new Uint8Array([
      0x52, 0x49, 0x46, 0x46, 0x28, 0x00, 0x00, 0x00, 0x57, 0x41, 0x56, 0x45,
      0x66, 0x6d, 0x74, 0x20, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00,
      0x44, 0xac, 0x00, 0x00, 0x88, 0x58, 0x01, 0x00, 0x02, 0x00, 0x10, 0x00,
      0x64, 0x61, 0x74, 0x61, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ]);
    return "data:audio/wav;base64," + btoa(String.fromCharCode(...b));
  }

  function formatTime(sec: number): string {
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60).toString().padStart(2, "0");
    return `${m}:${s}`;
  }

  function applyRegionLoop(region: Region): void {
    setLoop(region.start, region.end);
    hasLoop = true;
    // 현재 위치가 구간 밖이면 구간 시작으로 이동 (Transport 루프는 통과 시에만 wrap)
    const pos = getPosition();
    if (pos < region.start || pos > region.end) {
      seek(region.start);
    }
  }

  function removeLoop(): void {
    loopRegion?.remove();
    loopRegion = null;
    clearLoop();
    hasLoop = false;
  }

  onMount(() => {
    const peaks = getPeaks() ?? [0];
    regions = RegionsPlugin.create();

    ws = WaveSurfer.create({
      container,
      url: silentWavUri(),
      peaks: [peaks],
      duration,
      waveColor: "#8888aa",
      progressColor: "#8888aa",
      cursorWidth: 0,
      height: 80,
      barWidth: 2,
      barGap: 1,
      interact: true,
      plugins: [regions],
    });

    // 클릭 = 시크 (audio.ts Transport로 위임)
    ws.on("interaction", (newTime: number) => {
      seek(newTime);
      positionSec = newTime;
    });

    // 드래그 = A-B 구간 생성 (UI.md A-B Loop)
    regions.enableDragSelection({ color: "rgba(124, 92, 252, 0.2)" });
    regions.on("region-created", (region: Region) => {
      if (loopRegion && loopRegion.id !== region.id) {
        loopRegion.remove(); // 구간은 항상 1개만
      }
      loopRegion = region;
      applyRegionLoop(region);
    });
    regions.on("region-updated", (region: Region) => {
      if (region.id === loopRegion?.id) applyRegionLoop(region);
    });

    const tick = () => {
      positionSec = getPosition();
      checkEnded();
      rafId = requestAnimationFrame(tick);
    };
    rafId = requestAnimationFrame(tick);
  });

  onDestroy(() => {
    cancelAnimationFrame(rafId);
    ws?.destroy();
    ws = null;
  });

  let progressPct = $derived(
    duration > 0 ? Math.min(100, (positionSec / duration) * 100) : 0,
  );
</script>

<div class="flex flex-col gap-2">
  <div class="relative">
    <div bind:this={container}></div>
    <!-- 재생 커서 오버레이 (wavesurfer media 미사용 → 직접 렌더) -->
    <div
      class="pointer-events-none absolute top-0 h-full w-0.5 bg-accent"
      style="left: {progressPct}%"
    ></div>
  </div>

  <div class="flex items-center justify-between text-xs text-muted">
    <span>{formatTime(positionSec)} / {formatTime(duration)}</span>
    <div class="flex items-center gap-2">
      {#if hasLoop}
        <span class="text-accent">🔁 구간 반복 중</span>
        <button
          type="button"
          class="rounded-md border border-border px-2 py-0.5 transition-colors duration-200 hover:border-accent/40 hover:text-text"
          onclick={removeLoop}
        >
          구간 해제
        </button>
      {:else}
        <span>파형을 드래그하면 구간 반복(A-B)이 설정돼요</span>
      {/if}
    </div>
  </div>
</div>
