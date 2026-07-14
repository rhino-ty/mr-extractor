<script lang="ts">
  // Design Ref: ProcessPage Plan FR-11 — 카드 1개
  // (라벨 + duration + 진행률 바 + step 텍스트 + cancel [✕] + 완료 시 [열기 →])
  // queue-page FileCard 패턴 답습 — 코드 복제로 결합도 낮춤 (Plan §7.2 결정)

  import type { QueueItem, QueueItemStatus } from "$lib/types";
  import { formatDuration } from "$lib/queue";

  interface Props {
    item: QueueItem;
    selected: boolean;
    onSelect: (e: MouseEvent | KeyboardEvent) => void;
    onCancel: () => void;
    onOpen: () => void;
  }

  let { item, selected, onSelect, onCancel, onOpen }: Props = $props();

  // ProcessPage에서 실제 등장하는 상태만 의미 있음 — 나머지는 안전 fallback
  const STATUS_BADGES: Partial<Record<QueueItemStatus, { icon: string; label: string; tone: string }>> = {
    "ready-to-separate": { icon: "○", label: "대기 중", tone: "text-muted" },
    "in-progress": { icon: "▶", label: "분리 중...", tone: "text-accent" },
    done: { icon: "✓", label: "완료", tone: "text-success" },
    error: { icon: "❌", label: "오류", tone: "text-danger" },
  };

  let badge = $derived(
    STATUS_BADGES[item.status] ?? { icon: "○", label: "대기 중", tone: "text-muted" },
  );
  let isActive = $derived(item.status === "in-progress");
  let durationLabel = $derived(formatDuration(item.durationSec));

  function handleCancelClick(e: MouseEvent) {
    e.stopPropagation();
    onCancel();
  }

  function handleOpenClick(e: MouseEvent) {
    e.stopPropagation();
    onOpen();
  }
</script>

<div
  class="group flex w-full items-center gap-3 rounded-xl border p-4 transition-all duration-200 cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-accent {selected
    ? 'border-accent bg-accent/20'
    : 'border-border bg-surface hover:border-accent/40 hover:bg-bg/50'}"
  role="button"
  tabindex="0"
  aria-pressed={selected}
  onclick={onSelect}
  onkeydown={(e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onSelect(e);
    }
  }}
>
  <span class="text-xl shrink-0" aria-hidden="true">🎧</span>

  <div class="flex min-w-0 flex-1 flex-col gap-1">
    <div class="flex items-center gap-2">
      <span class="truncate text-sm text-text" title={item.label}>{item.label}</span>
      {#if durationLabel}
        <span class="shrink-0 text-xs text-muted">({durationLabel})</span>
      {/if}
    </div>

    <div class="flex items-center gap-2 text-xs">
      <span class={badge.tone}>{badge.icon}</span>
      <span class={badge.tone}>
        {isActive ? (item.step ?? badge.label) : badge.label}
        {#if isActive}
          <span class="ml-1 text-muted">{item.progress}%</span>
        {/if}
      </span>
    </div>

    {#if isActive}
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

  {#if isActive}
    <button
      type="button"
      class="shrink-0 rounded-md p-1 text-muted transition-colors duration-200 hover:bg-danger/20 hover:text-danger"
      aria-label="취소"
      title="취소 (Delete)"
      onclick={handleCancelClick}
    >
      ✕
    </button>
  {:else if item.status === "done"}
    <button
      type="button"
      class="shrink-0 rounded-lg bg-success px-3 py-1.5 text-xs font-medium text-bg transition-colors duration-200 hover:bg-success/80"
      onclick={handleOpenClick}
    >
      열기 →
    </button>
  {/if}
</div>
