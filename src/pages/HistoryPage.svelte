<script lang="ts">
  // Design Ref: HISTORY.md — 처리 히스토리 목록 + 다중 선택 + 삭제 다이얼로그.
  // 선택 패턴: queue-page 답습 (일반 클릭 토글 / Ctrl 개별 / Shift 범위).
  // 삭제: 기록만 vs 기록+파일 라디오 다이얼로그 (HISTORY.md 삭제 다이얼로그).

  import { onDestroy, onMount } from "svelte";
  import { fade } from "svelte/transition";
  import { open as shellOpen } from "@tauri-apps/plugin-shell";
  import { navigateTo, pushToast } from "$lib/stores";
  import { historyList, historyRemove } from "$lib/commands";
  import { addToQueue } from "$lib/queue";
  import { loadItem } from "$lib/audio";
  import type { HistoryEntryView } from "$lib/types";
  import HistoryCard from "../components/history/HistoryCard.svelte";

  let entries = $state<HistoryEntryView[]>([]);
  let loading = $state(true);
  // UX M1 — 로드 실패를 빈 상태와 구분 (에러인데 "히스토리가 없어요"로 보이지 않게)
  let loadError = $state(false);
  let selectedIds = $state<Set<string>>(new Set());
  let lastClickedId = $state<string | null>(null);

  // 삭제 다이얼로그 상태 — null이면 닫힘
  let deleteTarget = $state<string[] | null>(null);
  let deleteFiles = $state(false);

  let selectedCount = $derived(selectedIds.size);

  async function reload(): Promise<void> {
    try {
      entries = await historyList();
      loadError = false;
    } catch (err) {
      loadError = true;
      pushToast(typeof err === "string" ? err : String(err), "error");
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void reload();
    window.addEventListener("keydown", handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleKeydown);
  });

  // ─── 선택 (queue-page 패턴) ───────────────────────────────────────────────

  function handleSelect(id: string, e: MouseEvent | KeyboardEvent): void {
    const next = new Set(selectedIds);
    if (e.shiftKey && lastClickedId) {
      const ids = entries.map((it) => it.id);
      const a = ids.indexOf(lastClickedId);
      const b = ids.indexOf(id);
      if (a >= 0 && b >= 0) {
        const [from, to] = a < b ? [a, b] : [b, a];
        for (let i = from; i <= to; i++) next.add(ids[i]);
      }
    } else if (e.ctrlKey || e.metaKey) {
      if (next.has(id)) next.delete(id);
      else next.add(id);
    } else {
      if (next.size === 1 && next.has(id)) {
        next.clear();
      } else {
        next.clear();
        next.add(id);
      }
    }
    selectedIds = next;
    lastClickedId = id;
  }

  // ─── 액션 (HISTORY.md 항목 액션) ──────────────────────────────────────────

  function playEntry(view: HistoryEntryView): void {
    if (!view.stemsExist || !view.files.stems) return;
    // 히스토리는 queueStore에 없으므로 직접 로드 후 payload 없이 이동
    void loadItem({ id: view.id, label: view.title, outputs: view.files.stems });
    navigateTo("player");
  }

  function openFolder(view: HistoryEntryView): void {
    const dir = view.outDir || null;
    if (!dir) return;
    void shellOpen(dir).catch(() => pushToast("폴더를 열 수 없어요.", "error"));
  }

  function requeueEntry(view: HistoryEntryView): void {
    const item = addToQueue({
      sourceType: view.sourceType,
      source: view.source,
      label: view.title,
    });
    if (item) {
      pushToast("큐에 다시 추가했어요.", "success");
      navigateTo("queue");
    }
  }

  function askDelete(ids: string[]): void {
    if (ids.length === 0) return;
    deleteFiles = false;
    deleteTarget = ids;
  }

  async function confirmDelete(): Promise<void> {
    if (!deleteTarget) return;
    const ids = deleteTarget;
    deleteTarget = null;
    try {
      await historyRemove(ids, deleteFiles);
      selectedIds = new Set();
      await reload();
      pushToast(
        deleteFiles ? "기록과 파일을 삭제했어요." : "기록을 삭제했어요.",
        "success",
      );
    } catch (err) {
      pushToast(typeof err === "string" ? err : String(err), "error");
    }
  }

  // ─── 단축키 (SHORTCUTS.md 큐/히스토리 스코프) ─────────────────────────────

  function isTypingTarget(target: EventTarget | null): boolean {
    if (!(target instanceof HTMLElement)) return false;
    const tag = target.tagName;
    return tag === "INPUT" || tag === "TEXTAREA" || target.isContentEditable;
  }

  function handleKeydown(e: KeyboardEvent): void {
    if (isTypingTarget(e.target)) return;

    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "a") {
      e.preventDefault();
      selectedIds = new Set(entries.map((it) => it.id));
    } else if (e.key === "Delete" || e.key === "Backspace") {
      if (deleteTarget) return;
      if (selectedCount > 0) {
        e.preventDefault();
        askDelete([...selectedIds]);
      }
    } else if (e.key === "Escape") {
      e.preventDefault();
      if (deleteTarget) {
        deleteTarget = null;
      } else if (selectedCount > 0) {
        selectedIds = new Set();
      } else {
        navigateTo("queue");
      }
    }
  }
</script>

<div class="flex h-full flex-col gap-4 p-6">
  <div class="flex items-center justify-between">
    <h2 class="text-xl font-bold text-text">처리 히스토리</h2>
    {#if entries.length > 0}
      <button
        type="button"
        class="rounded-lg border border-danger/40 bg-danger/10 px-3 py-1.5 text-sm text-danger transition-colors duration-200 hover:bg-danger/20"
        onclick={() => askDelete(entries.map((it) => it.id))}
      >
        🗑 전체 삭제
      </button>
    {/if}
  </div>

  <div class="flex-1 overflow-y-auto">
    {#if loading}
      <div class="flex h-full items-center justify-center" in:fade={{ duration: 200 }}>
        <div
          class="h-8 w-8 animate-spin rounded-full border-4 border-border border-t-accent"
          aria-hidden="true"
        ></div>
      </div>
    {:else if loadError}
      <div
        class="flex h-full flex-col items-center justify-center gap-3 text-center"
        in:fade={{ duration: 200 }}
      >
        <div class="text-5xl" aria-hidden="true">❌</div>
        <p class="text-lg font-semibold text-text">히스토리를 불러올 수 없어요</p>
        <button
          type="button"
          class="mt-2 rounded-lg bg-accent px-4 py-2 text-sm font-medium text-white transition-colors duration-200 hover:bg-accent/80"
          onclick={() => {
            loading = true;
            void reload();
          }}
        >
          🔄 다시 시도
        </button>
      </div>
    {:else if entries.length === 0}
      <div
        class="flex h-full flex-col items-center justify-center gap-3 text-center"
        in:fade={{ duration: 200 }}
      >
        <div class="text-5xl" aria-hidden="true">🕐</div>
        <p class="text-lg font-semibold text-text">아직 히스토리가 없어요</p>
        <p class="text-sm text-muted">분리를 완료하면 여기에 기록이 남아요</p>
      </div>
    {:else}
      <div class="flex flex-col gap-2">
        {#each entries as view (view.id)}
          <div in:fade={{ duration: 150 }}>
            <HistoryCard
              {view}
              selected={selectedIds.has(view.id)}
              onSelect={(e) => handleSelect(view.id, e)}
              onPlay={() => playEntry(view)}
              onOpenFolder={() => openFolder(view)}
              onRequeue={() => requeueEntry(view)}
              onDelete={() => askDelete([view.id])}
            />
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <!-- 하단 액션 바 (다중 선택) -->
  {#if selectedCount > 0}
    <div
      class="flex items-center justify-between gap-3 rounded-xl border border-border bg-surface px-4 py-3"
      in:fade={{ duration: 150 }}
    >
      <span class="text-sm text-muted">{selectedCount}개 선택됨</span>
      <button
        type="button"
        class="rounded-lg border border-danger/40 bg-danger/10 px-4 py-2 text-sm font-medium text-danger transition-colors duration-200 hover:bg-danger/20"
        onclick={() => askDelete([...selectedIds])}
      >
        🗑 선택 삭제 ({selectedCount})
      </button>
    </div>
  {/if}
</div>

<!-- 삭제 옵션 다이얼로그 (HISTORY.md) -->
{#if deleteTarget}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-bg/70"
    transition:fade={{ duration: 150 }}
  >
    <div class="w-96 rounded-xl border border-border bg-surface p-5 shadow-xl" role="dialog" aria-modal="true">
      <h3 class="text-sm font-semibold text-text">
        {deleteTarget.length}개 항목 삭제
      </h3>
      <div class="mt-4 flex flex-col gap-2 text-sm text-text">
        <label class="flex cursor-pointer items-center gap-2">
          <input
            type="radio"
            name="delete-mode"
            class="accent-accent"
            checked={!deleteFiles}
            onchange={() => (deleteFiles = false)}
          />
          기록만 삭제 (파일 유지)
        </label>
        <label class="flex cursor-pointer items-center gap-2">
          <input
            type="radio"
            name="delete-mode"
            class="accent-accent"
            checked={deleteFiles}
            onchange={() => (deleteFiles = true)}
          />
          기록 + 파일 모두 삭제
        </label>
        {#if deleteFiles}
          <p class="text-xs text-warn">
            스템과 내보낸 반주 파일이 함께 삭제돼요. 되돌릴 수 없어요.
          </p>
        {/if}
      </div>
      <div class="mt-5 flex justify-end gap-2">
        <button
          type="button"
          class="rounded-lg border border-border px-4 py-2 text-sm text-muted transition-colors duration-200 hover:text-text"
          onclick={() => (deleteTarget = null)}
        >
          취소
        </button>
        <button
          type="button"
          class="rounded-lg bg-danger px-4 py-2 text-sm font-medium text-white transition-colors duration-200 hover:bg-danger/80"
          onclick={() => void confirmDelete()}
        >
          🗑 삭제
        </button>
      </div>
    </div>
  </div>
{/if}
