<script lang="ts">
  // Design Ref: §5.1 / §11.2 — QueuePage 조립 (DropZone + UrlInput + ModelSelector slot + 큐 + 액션 바)
  // Plan FR-03/FR-04/FR-05/FR-19/FR-21 — 다중 선택 + 일괄 삭제 + 분리 시작 + 단축키

  import { onMount, onDestroy } from "svelte";
  import { get } from "svelte/store";
  import { fade } from "svelte/transition";
  import { queueStore, isProcessing, navigateTo, pushToast, selectedModel } from "$lib/stores";
  import {
    classifyFile,
    removeFromQueue,
    removeManyFromQueue,
    updateQueueItem,
  } from "$lib/queue";
  import { downloadYoutube, extractAudio, fetchVideoMetadata } from "$lib/commands";
  import type { QueueItem } from "$lib/types";
  import DropZone from "../components/queue/DropZone.svelte";
  import UrlInput from "../components/queue/UrlInput.svelte";
  import FileCard from "../components/queue/FileCard.svelte";
  import EmptyState from "../components/queue/EmptyState.svelte";
  import ModelSelector from "../components/queue/ModelSelector.svelte";

  // ─── 선택 상태 (Plan FR-03) ──────────────────────────────────────────────
  let selectedIds = $state<Set<string>>(new Set());
  let lastClickedId = $state<string | null>(null);

  let queue = $derived($queueStore);
  let selectedCount = $derived(selectedIds.size);

  // Phase 3 — pending file/video/youtube 항목 1개 이상 선택 + 다른 항목이 처리 중이 아닐 때 활성화
  // 처리 중 재클릭 방지 (사용자 요청 — 중복 처리 방지)
  let canStart = $derived.by(() => {
    if (selectedIds.size === 0) return false;
    if ($isProcessing) return false;
    return queue.some(
      (it) => selectedIds.has(it.id) && it.status === "pending",
    );
  });

  // 큐 변경 시 사라진 ID는 selection에서도 제거
  $effect(() => {
    const valid = new Set(queue.map((it) => it.id));
    let changed = false;
    const next = new Set<string>();
    for (const id of selectedIds) {
      if (valid.has(id)) {
        next.add(id);
      } else {
        changed = true;
      }
    }
    if (changed) selectedIds = next;
  });

  // ─── 선택 핸들러 ─────────────────────────────────────────────────────────

  function handleSelect(item: { id: string }, e: MouseEvent | KeyboardEvent): void {
    const id = item.id;
    const next = new Set(selectedIds);

    if (e.shiftKey && lastClickedId) {
      // Shift+클릭 = 범위 선택
      const ids = queue.map((it) => it.id);
      const a = ids.indexOf(lastClickedId);
      const b = ids.indexOf(id);
      if (a >= 0 && b >= 0) {
        const [from, to] = a < b ? [a, b] : [b, a];
        for (let i = from; i <= to; i++) next.add(ids[i]);
      }
    } else if (e.ctrlKey || e.metaKey) {
      // Ctrl+클릭 = 개별 토글
      if (next.has(id)) next.delete(id);
      else next.add(id);
    } else {
      // 일반 클릭 = 단일 선택 토글
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

  // ─── 액션 ─────────────────────────────────────────────────────────────────

  function selectAll(): void {
    selectedIds = new Set(queue.map((it) => it.id));
  }

  function clearSelection(): void {
    selectedIds = new Set();
    lastClickedId = null;
  }

  function deleteSelected(): void {
    if (selectedCount === 0) return;
    const ids = [...selectedIds];
    void removeManyFromQueue(ids);
    clearSelection();
  }

  // Plan FR-05 / Design 시나리오 C — Phase 3: 모든 sourceType 처리 + ProcessPage 라우팅
  async function startSeparation(): Promise<void> {
    if (!canStart || selectedCount === 0) return;
    const selected = queue.filter(
      (it) => selectedIds.has(it.id) && it.status === "pending",
    );
    if (selected.length === 0) return;

    pushToast(`${selected.length}개 항목 처리를 시작했어요.`, "success");

    // 항목별 병렬 처리 (모두 settle될 때까지 대기)
    await Promise.allSettled(selected.map((it) => processItem(it)));

    // 처리 완료 후 ready-to-separate 도달한 항목들로 ProcessPage 라우팅
    const finalQueue = get(queueStore);
    const readyIds = selected
      .map((it) => it.id)
      .filter(
        (id) => finalQueue.find((q) => q.id === id)?.status === "ready-to-separate",
      );
    if (readyIds.length === 0) return;

    // v1.1 ModelSelector — 선택된 모델 전달 (기본 htdemucs_ft)
    navigateTo("process", { kind: "process", ids: readyIds, model: get(selectedModel) });
  }

  async function processItem(item: QueueItem): Promise<void> {
    if (item.sourceType === "file") {
      // 오디오 파일: 변환 불필요 — tmpPath = source 그대로
      updateQueueItem(item.id, {
        status: "ready-to-separate",
        tmpPath: item.source,
        progress: 100,
        step: undefined,
      });
      return;
    }

    if (item.sourceType === "video") {
      if (!item.durationSec) {
        pushToast("아직 영상 정보를 가져오는 중이에요", "info");
        return;
      }
      return runExtraction(item.id, item.source, item.durationSec);
    }

    if (item.sourceType === "youtube") {
      return runYoutubeDownload(item.id, item.source, item.durationSec);
    }
  }

  async function runExtraction(
    id: string,
    path: string,
    durationSec: number,
  ): Promise<void> {
    updateQueueItem(id, {
      status: "extracting",
      step: "오디오 추출 중...",
      progress: 0,
    });
    try {
      const tmpPath = await extractAudio(id, path, durationSec, (p) => {
        updateQueueItem(id, { progress: p.percent });
      });
      updateQueueItem(id, {
        status: "ready-to-separate",
        tmpPath,
        progress: 100,
        step: undefined,
      });
    } catch (err) {
      const msg = typeof err === "string" ? err : String(err);
      updateQueueItem(id, {
        status: "error",
        errorDetail: msg,
        step: undefined,
      });
      pushToast(msg, "error");
    }
  }

  // Plan FR-07 / 시나리오 C — youtube 다운로드 후 영상이면 추출 체이닝
  async function runYoutubeDownload(
    id: string,
    url: string,
    knownDuration: number | undefined,
  ): Promise<void> {
    updateQueueItem(id, {
      status: "downloading",
      step: "다운로드 중...",
      progress: 0,
    });
    try {
      const downloadedPath = await downloadYoutube(id, url, (p) => {
        updateQueueItem(id, { progress: p.percent, step: p.step });
      });

      // 다운로드 결과 ext 분류 — audio면 그대로, video면 추출 체이닝
      const kind = classifyFile(downloadedPath);
      if (kind === "audio") {
        updateQueueItem(id, {
          status: "ready-to-separate",
          tmpPath: downloadedPath,
          progress: 100,
          step: undefined,
        });
        return;
      }

      // video → ffprobe + extract_audio
      let durationSec = knownDuration ?? 0;
      if (durationSec <= 0) {
        try {
          const meta = await fetchVideoMetadata(id, downloadedPath);
          durationSec = meta.durationSec;
        } catch {
          durationSec = 1; // fallback (진행률 계산만 부정확, 추출 자체는 진행)
        }
      }
      await runExtraction(id, downloadedPath, durationSec);
    } catch (err) {
      const msg = typeof err === "string" ? err : String(err);
      updateQueueItem(id, {
        status: "error",
        errorDetail: msg,
        step: undefined,
      });
      pushToast(msg, "error");
    }
  }

  // ─── 단축키 (Plan FR-21) ─────────────────────────────────────────────────
  // Ctrl+O는 Iterate 1 (I-2)에서 App.svelte 전역 핸들러로 이전 (모든 페이지에서 동작).
  // QueuePage 로컬 핸들러는 큐 컨텍스트 단축키만 (Ctrl+A / Delete / Backspace / Escape).

  function isTypingTarget(target: EventTarget | null): boolean {
    if (!(target instanceof HTMLElement)) return false;
    const tag = target.tagName;
    return tag === "INPUT" || tag === "TEXTAREA" || target.isContentEditable;
  }

  function handleKeydown(e: KeyboardEvent): void {
    // 입력 중에는 단축키 무시
    if (isTypingTarget(e.target)) return;

    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "a") {
      e.preventDefault();
      selectAll();
    } else if (e.key === "Delete" || e.key === "Backspace") {
      if (selectedCount > 0) {
        e.preventDefault();
        deleteSelected();
      }
    } else if (e.key === "Escape") {
      if (selectedCount > 0) {
        e.preventDefault();
        clearSelection();
      }
    }
  }

  onMount(() => {
    window.addEventListener("keydown", handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleKeydown);
  });
</script>

<div class="flex h-full flex-col gap-4 p-6">
  <!-- 상단 입력 영역 -->
  <div class="flex flex-col gap-3">
    <div class="grid grid-cols-1 gap-3 md:grid-cols-2">
      <DropZone />
      <UrlInput />
    </div>
    <!-- v1.1 ModelSelector (reserve 슬롯 본구현) -->
    <div data-slot="model-selector">
      <ModelSelector />
    </div>
  </div>

  <!-- 큐 영역 -->
  <div class="flex-1 overflow-y-auto">
    {#if queue.length === 0}
      <div in:fade={{ duration: 200 }}>
        <EmptyState />
      </div>
    {:else}
      <div class="flex flex-col gap-2">
        {#each queue as item (item.id)}
          <div in:fade={{ duration: 150 }}>
            <FileCard
              {item}
              selected={selectedIds.has(item.id)}
              onSelect={(e) => handleSelect(item, e)}
              onRemove={() => void removeFromQueue(item.id)}
              onOpen={() =>
                navigateTo("player", { kind: "player", itemId: item.id })}
            />
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <!-- 하단 액션 바 -->
  {#if selectedCount > 0}
    <div
      class="flex items-center justify-between gap-3 rounded-xl border border-border bg-surface px-4 py-3"
      in:fade={{ duration: 150 }}
    >
      <span class="text-sm text-muted">{selectedCount}개 선택됨</span>
      <div class="flex gap-2">
        <button
          type="button"
          class="rounded-lg border border-danger/40 bg-danger/10 px-4 py-2 text-sm font-medium text-danger transition-colors hover:bg-danger/20"
          onclick={deleteSelected}
        >
          🗑 삭제 ({selectedCount})
        </button>
        <button
          type="button"
          class="rounded-lg bg-accent px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-accent/80 disabled:cursor-not-allowed disabled:opacity-50"
          onclick={() => void startSeparation()}
          disabled={!canStart}
          title={canStart
            ? "선택 항목 분리 시작"
            : $isProcessing
              ? "처리 중이에요. 완료 후 다시 시도해주세요."
              : "처리할 항목을 선택해주세요"}
        >
          ▶ 분리 시작 ({selectedCount})
        </button>
      </div>
    </div>
  {/if}
</div>
