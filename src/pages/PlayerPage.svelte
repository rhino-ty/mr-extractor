<script lang="ts">
  // Design Ref: UI.md PlayerPage 레이아웃 — 파형 + 트랜스포트 + 스템 믹서 + 키 조절.
  // 오디오 그래프는 $lib/audio 싱글턴 (페이지 unmount에도 유지 — 재진입 시 그대로).
  // 단축키: SHORTCUTS.md (Space/←→/Shift+←→/↑↓/M/Escape)

  import { onDestroy, onMount } from "svelte";
  import { get } from "svelte/store";
  import { fade } from "svelte/transition";
  import { queueStore, pagePayload, navigateTo, goBack } from "$lib/stores";
  import {
    isPlaying,
    loadItem,
    loadedTrack,
    mixerStore,
    nudgeMasterVolume,
    getPosition,
    seek,
    setSemitones,
    stop,
    toggleMuteAll,
    togglePlay,
  } from "$lib/audio";
  import WaveformPlayer from "../components/player/WaveformPlayer.svelte";
  import StemMixer from "../components/player/StemMixer.svelte";
  import ExportPanel from "../components/player/ExportPanel.svelte";

  let track = $derived($loadedTrack);
  let playing = $derived($isPlaying);
  let semitones = $derived($mixerStore.semitones);

  onMount(() => {
    const payload = get(pagePayload);
    if (payload?.kind === "player") {
      const item = get(queueStore).find((it) => it.id === payload.itemId);
      if (item) {
        void loadItem(item);
      }
    }
    // payload가 없으면(goBack 재진입) 기존 loadedTrack 상태 그대로 표시
    window.addEventListener("keydown", handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleKeydown);
  });

  // ─── 단축키 (SHORTCUTS.md PlayerPage 스코프) ───────────────────────────────

  function isTypingTarget(target: EventTarget | null): boolean {
    if (!(target instanceof HTMLElement)) return false;
    const tag = target.tagName;
    return tag === "INPUT" || tag === "TEXTAREA" || target.isContentEditable;
  }

  function handleKeydown(e: KeyboardEvent): void {
    if (isTypingTarget(e.target)) return;

    if (e.key === " ") {
      e.preventDefault();
      void togglePlay();
    } else if (e.key === "ArrowLeft") {
      e.preventDefault();
      seek(getPosition() - (e.shiftKey ? 30 : 5));
    } else if (e.key === "ArrowRight") {
      e.preventDefault();
      seek(getPosition() + (e.shiftKey ? 30 : 5));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      nudgeMasterVolume(5);
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      nudgeMasterVolume(-5);
    } else if (e.key.toLowerCase() === "m" && !e.ctrlKey && !e.altKey) {
      e.preventDefault();
      toggleMuteAll();
    } else if (e.key === "Escape") {
      e.preventDefault();
      goBack();
    }
  }

  function keyLabel(n: number): string {
    if (n === 0) return "원래 키";
    return n > 0 ? `+${n}반음` : `${n}반음`;
  }
</script>

<div class="flex h-full flex-col gap-4 p-6">
  {#if track.status === "idle"}
    <div class="flex flex-1 flex-col items-center justify-center gap-3 text-center" in:fade={{ duration: 200 }}>
      <div class="text-5xl" aria-hidden="true">🎧</div>
      <p class="text-lg font-semibold text-text">재생할 곡이 없어요</p>
      <p class="text-sm text-muted">분리가 완료된 항목에서 [열기 →]를 눌러주세요</p>
      <button
        type="button"
        class="mt-2 rounded-lg bg-accent px-4 py-2 text-sm font-medium text-white transition-colors duration-200 hover:bg-accent/80"
        onclick={() => navigateTo("queue")}
      >
        큐로 돌아가기
      </button>
    </div>
  {:else if track.status === "loading"}
    <div class="flex flex-1 flex-col items-center justify-center gap-4" in:fade={{ duration: 200 }}>
      <div
        class="h-10 w-10 animate-spin rounded-full border-4 border-border border-t-accent"
        aria-hidden="true"
      ></div>
      <p class="text-sm text-muted">「{track.label}」 스템을 불러오는 중...</p>
    </div>
  {:else if track.status === "error"}
    <div class="flex flex-1 flex-col items-center justify-center gap-3 text-center" in:fade={{ duration: 200 }}>
      <div class="text-5xl" aria-hidden="true">❌</div>
      <p class="text-lg font-semibold text-text">곡을 불러올 수 없어요</p>
      {#if track.error}
        <p class="max-w-md text-sm text-danger">{track.error}</p>
      {/if}
      <button
        type="button"
        class="mt-2 rounded-lg bg-accent px-4 py-2 text-sm font-medium text-white transition-colors duration-200 hover:bg-accent/80"
        onclick={() => navigateTo("queue")}
      >
        큐로 돌아가기
      </button>
    </div>
  {:else}
    <!-- 타이틀 (UI.md 레이아웃 — ← 목록 + 곡 제목) -->
    <div class="flex items-center gap-3" in:fade={{ duration: 200 }}>
      <button
        type="button"
        class="shrink-0 rounded-lg border border-border px-3 py-1.5 text-sm text-muted transition-colors duration-200 hover:border-accent/40 hover:text-text"
        title="큐로 돌아가기 (Escape)"
        onclick={() => navigateTo("queue")}
      >
        ← 목록
      </button>
      <h2 class="truncate text-xl font-bold text-text" title={track.label}>
        {track.label}
      </h2>
    </div>

    <!-- 파형 + A-B 구간 (곡 바뀌면 재마운트) -->
    {#key track.itemId}
      <div class="rounded-xl border border-border bg-surface p-4">
        <WaveformPlayer />
      </div>
    {/key}

    <!-- 트랜스포트 -->
    <div class="flex items-center justify-center gap-3">
      <button
        type="button"
        class="rounded-lg border border-border px-4 py-2 text-sm text-muted transition-colors duration-200 hover:border-accent/40 hover:text-text"
        title="처음으로"
        onclick={() => seek(0)}
      >
        ◀◀
      </button>
      <button
        type="button"
        class="rounded-lg bg-accent px-8 py-2 text-sm font-medium text-white transition-colors duration-200 hover:bg-accent/80"
        title="재생/일시정지 (Space)"
        onclick={() => void togglePlay()}
      >
        {playing ? "⏸ 일시정지" : "▶ 재생"}
      </button>
      <button
        type="button"
        class="rounded-lg border border-border px-4 py-2 text-sm text-muted transition-colors duration-200 hover:border-accent/40 hover:text-text"
        title="정지"
        onclick={() => stop()}
      >
        ■ 정지
      </button>
    </div>

    <!-- 스템 믹서 (M = 전체 뮤트 토글) -->
    <StemMixer />

    <!-- 키 조절 (Tone.js 실시간 미리듣기 / 내보내기 시 동일 값 적용) -->
    <div class="flex items-center gap-3 rounded-xl border border-border bg-surface p-4">
      <span class="w-20 shrink-0 text-sm font-semibold text-text">
        <span aria-hidden="true">🎼</span>
        키 조절
      </span>
      <button
        type="button"
        class="shrink-0 rounded-md border border-border px-2 py-0.5 text-sm text-muted transition-colors duration-200 hover:border-accent/40 hover:text-text"
        aria-label="반음 내리기"
        onclick={() => setSemitones(semitones - 1)}
      >
        −
      </button>
      <input
        type="range"
        min="-12"
        max="12"
        step="1"
        value={semitones}
        class="h-1.5 flex-1 cursor-pointer accent-accent"
        aria-label="키 조절 (반음)"
        oninput={(e) => setSemitones(Number(e.currentTarget.value))}
      />
      <button
        type="button"
        class="shrink-0 rounded-md border border-border px-2 py-0.5 text-sm text-muted transition-colors duration-200 hover:border-accent/40 hover:text-text"
        aria-label="반음 올리기"
        onclick={() => setSemitones(semitones + 1)}
      >
        +
      </button>
      <span class="w-16 shrink-0 text-right text-sm {semitones === 0 ? 'text-muted' : 'text-accent'}">
        {keyLabel(semitones)}
      </span>
      {#if semitones !== 0}
        <button
          type="button"
          class="shrink-0 rounded-md border border-border px-2 py-0.5 text-xs text-muted transition-colors duration-200 hover:border-accent/40 hover:text-text"
          onclick={() => setSemitones(0)}
        >
          초기화
        </button>
      {/if}
    </div>

    <!-- 내보내기 (현재 믹스 + 키 조절 반영, ~/Desktop/MR Extractor/) -->
    {#if track.outputs}
      <ExportPanel outputs={track.outputs} />
    {/if}
  {/if}
</div>
