<script lang="ts">
  // Design Ref: HISTORY.md UI 레이아웃 — 항목 1개 (아이콘+제목+날짜+뱃지+액션 버튼).
  // 뱃지 3종: 🎚 스템 있음(accent) / 🎵 반주만(muted) / ⚠ 파일 없음(warn).

  import type { HistoryEntryView } from "$lib/types";

  interface Props {
    view: HistoryEntryView;
    selected: boolean;
    onSelect: (e: MouseEvent | KeyboardEvent) => void;
    onPlay: () => void;
    onOpenFolder: () => void;
    onRequeue: () => void;
    onDelete: () => void;
  }

  let { view, selected, onSelect, onPlay, onOpenFolder, onRequeue, onDelete }: Props =
    $props();

  const SOURCE_ICONS = { youtube: "🔗", file: "🎵", video: "🎬" } as const;
  const SOURCE_LABELS = { youtube: "YouTube", file: "파일", video: "영상" } as const;

  let dateLabel = $derived(view.date.slice(0, 10).replaceAll("-", "."));
  let failed = $derived(view.status === "error");

  function stopThen(fn: () => void): (e: MouseEvent) => void {
    return (e) => {
      e.stopPropagation();
      fn();
    };
  }
</script>

<div
  class="group flex w-full items-start gap-3 rounded-xl border p-4 transition-all duration-200 cursor-pointer focus:outline-none focus-visible:ring-2 focus-visible:ring-accent {selected
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
  <span class="text-xl shrink-0" aria-hidden="true">
    {SOURCE_ICONS[view.sourceType]}
  </span>

  <div class="flex min-w-0 flex-1 flex-col gap-1.5">
    <div class="flex items-center justify-between gap-2">
      <span class="truncate text-sm text-text" title={view.title}>{view.title}</span>
      <span class="shrink-0 text-xs text-muted">{dateLabel}</span>
    </div>

    <div class="flex items-center gap-2 text-xs text-muted">
      <span>{view.model}</span>
      <span>·</span>
      <span>{SOURCE_LABELS[view.sourceType]}</span>
      {#if failed}
        <span class="text-danger">· ❌ 실패</span>
      {/if}
    </div>

    <div class="flex items-center gap-2">
      {#if failed}
        {#if view.errorMsg}
          <span class="truncate text-xs text-danger" title={view.errorMsg}>
            {view.errorMsg}
          </span>
        {/if}
      {:else if view.stemsExist}
        <span class="rounded-md bg-accent/20 px-2 py-0.5 text-xs text-accent">
          🎚 스템 있음
        </span>
      {:else if view.instExists}
        <span
          class="rounded-md bg-bg/60 px-2 py-0.5 text-xs text-muted"
          title="재처리하면 스템을 다시 만들 수 있어요"
        >
          🎵 반주만
        </span>
      {:else}
        <span class="rounded-md bg-warn/10 px-2 py-0.5 text-xs text-warn">
          ⚠ 파일 없음
        </span>
      {/if}
    </div>

    <div class="mt-1 flex items-center gap-2">
      <button
        type="button"
        class="rounded-md border border-border px-2 py-1 text-xs text-muted transition-colors duration-200 hover:border-accent/40 hover:text-text disabled:cursor-not-allowed disabled:opacity-40"
        disabled={!view.stemsExist}
        title={view.stemsExist ? "믹서에서 열기" : "스템 파일이 없어요. 재처리해 주세요."}
        onclick={stopThen(onPlay)}
      >
        ▶ 재생
      </button>
      <button
        type="button"
        class="rounded-md border border-border px-2 py-1 text-xs text-muted transition-colors duration-200 hover:border-accent/40 hover:text-text disabled:cursor-not-allowed disabled:opacity-40"
        disabled={!view.stemsExist && !view.instExists}
        title="폴더 열기"
        onclick={stopThen(onOpenFolder)}
      >
        📁 폴더 열기
      </button>
      <button
        type="button"
        class="rounded-md border border-border px-2 py-1 text-xs text-muted transition-colors duration-200 hover:border-accent/40 hover:text-text"
        title="원본을 큐에 다시 추가"
        onclick={stopThen(onRequeue)}
      >
        🔄 재처리
      </button>
      <button
        type="button"
        class="ml-auto rounded-md border border-border px-2 py-1 text-xs text-muted transition-colors duration-200 hover:border-danger/60 hover:bg-danger/10 hover:text-danger"
        title="삭제"
        onclick={stopThen(onDelete)}
      >
        🗑
      </button>
    </div>
  </div>
</div>
