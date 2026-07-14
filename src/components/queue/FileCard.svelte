<script lang="ts">
  // Design Ref: §5.3 — 큐 항목 1개 (아이콘+라벨+길이+상태+진행률+✕)
  // Plan SC-20 — 처리 중 ⏳ 시각화 / fix #12 — hover/focus 시각 처리

  import type { QueueItem, QueueItemStatus } from "$lib/types";

  interface Props {
    item: QueueItem;
    selected: boolean;
    onSelect: (e: MouseEvent) => void;
    onRemove: () => void;
    /// UX H1 — 완료 항목 [열기 →] (분리 결과가 있을 때만 표시)
    onOpen?: () => void;
  }

  let { item, selected, onSelect, onRemove, onOpen }: Props = $props();

  const SOURCE_ICONS = {
    youtube: "🔗",
    file: "🎵",
    video: "🎬",
  } as const;

  const STATUS_BADGES: Record<QueueItemStatus, { icon: string; label: string; tone: string }> = {
    pending: { icon: "○", label: "대기", tone: "text-muted" },
    "fetching-metadata": { icon: "⏳", label: "정보 가져오는 중...", tone: "text-accent" },
    downloading: { icon: "⏳", label: "다운로드 중", tone: "text-accent" },
    extracting: { icon: "⏳", label: "오디오 추출 중", tone: "text-accent" },
    "ready-to-separate": { icon: "✅", label: "준비 완료", tone: "text-success" },
    "in-progress": { icon: "▶", label: "분리 중", tone: "text-accent" },
    done: { icon: "✓", label: "완료", tone: "text-success" },
    error: { icon: "❌", label: "오류", tone: "text-danger" },
  };

  function formatDuration(sec: number | undefined): string {
    if (!sec || sec <= 0) return "";
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60).toString().padStart(2, "0");
    return `${m}:${s}`;
  }

  let badge = $derived(STATUS_BADGES[item.status]);
  let isProcessing = $derived(
    item.status === "downloading" ||
      item.status === "extracting" ||
      item.status === "fetching-metadata",
  );
  let durationLabel = $derived(formatDuration(item.durationSec));

  function handleRemoveClick(e: MouseEvent) {
    e.stopPropagation();
    onRemove();
  }
</script>

<div
  class="group flex w-full items-center gap-3 rounded-xl border px-4 py-3 transition-all duration-100 cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-accent {selected
    ? 'border-accent bg-accent/20'
    : 'border-border bg-surface hover:border-accent/40 hover:bg-bg/50'}"
  role="button"
  tabindex="0"
  aria-pressed={selected}
  onclick={onSelect}
  onkeydown={(e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onSelect(e as unknown as MouseEvent);
    }
  }}
>
  <span class="text-xl shrink-0" aria-hidden="true">{SOURCE_ICONS[item.sourceType]}</span>

  <div class="flex min-w-0 flex-1 flex-col gap-1">
    <div class="flex items-center gap-2">
      <span class="truncate text-sm text-text" title={item.label}>{item.label}</span>
      {#if durationLabel}
        <span class="shrink-0 text-xs text-muted">({durationLabel})</span>
      {/if}
      {#if item.sourceType === "video"}
        <span class="shrink-0 text-xs text-muted">(영상에서 추출)</span>
      {/if}
    </div>

    <div class="flex items-center gap-2 text-xs">
      <span class={badge.tone}>{badge.icon}</span>
      <span class={badge.tone}>{item.step ?? badge.label}</span>
    </div>

    {#if isProcessing && item.progress > 0}
      <div class="h-1 w-full overflow-hidden rounded-full bg-bg">
        <div
          class="h-full bg-accent transition-all duration-200"
          style="width: {item.progress}%"
        ></div>
      </div>
    {/if}

    {#if item.status === "error" && item.errorDetail}
      <details class="text-xs text-danger">
        <summary class="cursor-pointer">상세</summary>
        <pre class="mt-1 max-h-32 overflow-auto rounded bg-bg/50 p-2 font-mono text-[11px] whitespace-pre-wrap">{item.errorDetail}</pre>
      </details>
    {/if}
  </div>

  {#if item.status === "done" && item.outputs && onOpen}
    <button
      type="button"
      class="shrink-0 rounded-lg bg-success px-3 py-1.5 text-xs font-medium text-bg transition-colors duration-200 hover:bg-success/80"
      title="믹서에서 열기"
      onclick={(e) => {
        e.stopPropagation();
        onOpen();
      }}
    >
      열기 →
    </button>
  {/if}
  <button
    type="button"
    class="shrink-0 rounded-md p-1 text-muted transition-colors duration-200 hover:bg-danger/20 hover:text-danger"
    aria-label="삭제"
    onclick={handleRemoveClick}
  >
    ✕
  </button>
</div>
