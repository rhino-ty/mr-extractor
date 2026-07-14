<script lang="ts">
  // Design Ref: UI.md PlayerPage — 스템별 [M] 뮤트 + 볼륨 슬라이더 + 마스터 볼륨.
  // 상태는 audio.ts mixerStore(SoT) — 여기서는 렌더 + 액션 위임만.

  import {
    STEM_META,
    STEM_ORDER,
    mixerStore,
    setMasterVolume,
    setStemMuted,
    setStemVolume,
  } from "$lib/audio";

  let mixer = $derived($mixerStore);
</script>

<div class="flex flex-col gap-2 rounded-xl border border-border bg-surface p-4">
  {#each STEM_ORDER as name (name)}
    {@const s = mixer.stems[name]}
    <div class="flex items-center gap-3">
      <span class="w-20 shrink-0 text-sm text-text">
        <span aria-hidden="true">{STEM_META[name].icon}</span>
        {STEM_META[name].label}
      </span>
      <button
        type="button"
        class="shrink-0 rounded-md border px-2 py-0.5 text-xs font-semibold transition-colors duration-200 {s.muted
          ? 'border-danger/60 bg-danger/20 text-danger'
          : 'border-border text-muted hover:border-accent/40 hover:text-text'}"
        aria-pressed={s.muted}
        title={s.muted ? "뮤트 해제" : "뮤트"}
        onclick={() => setStemMuted(name, !s.muted)}
      >
        M
      </button>
      <input
        type="range"
        min="0"
        max="100"
        value={s.volume}
        disabled={s.muted}
        class="h-1.5 flex-1 cursor-pointer accent-accent disabled:opacity-40"
        aria-label="{STEM_META[name].label} 볼륨"
        oninput={(e) => setStemVolume(name, Number(e.currentTarget.value))}
      />
      <span class="w-8 shrink-0 text-right text-xs text-muted">{s.volume}</span>
    </div>
  {/each}

  <div class="mt-2 flex items-center gap-3 border-t border-border pt-3">
    <span class="w-20 shrink-0 text-sm font-semibold text-text">
      <span aria-hidden="true">🔊</span>
      전체
    </span>
    <span class="w-[34px] shrink-0"></span>
    <input
      type="range"
      min="0"
      max="100"
      value={mixer.master}
      class="h-1.5 flex-1 cursor-pointer accent-accent"
      aria-label="마스터 볼륨 (↑/↓)"
      title="마스터 볼륨 (↑/↓)"
      oninput={(e) => setMasterVolume(Number(e.currentTarget.value))}
    />
    <span class="w-8 shrink-0 text-right text-xs text-muted">{mixer.master}</span>
  </div>
</div>
