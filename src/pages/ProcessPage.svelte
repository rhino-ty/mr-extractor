<script lang="ts">
  // Design Ref: ProcessPage Plan §2.1 Phase 1+3 — 카드 리스트 + queueStore subscribe
  //   + 순차 loop 트리거 + 단축키 (Escape=큐 복귀, Delete=선택 항목 취소)
  // Plan FR-01 (payload ids만 표시) / FR-08 (개별 cancel) / FR-12 (EmptyState) / FR-13 (단축키)
  //
  // 순차 처리 loop 본체는 $lib/process.ts (store layer) — 이 페이지는 트리거만.
  // unmount해도 loop와 진행률(queueStore SoT)은 계속 유지 (FR-10 / SC-10).

  import { onMount, onDestroy } from "svelte";
  import { get } from "svelte/store";
  import { fade } from "svelte/transition";
  import { queueStore, pagePayload, navigateTo } from "$lib/stores";
  import { cancelQueueItem } from "$lib/commands";
  import {
    startSeparationBatch,
    processBatch,
    isSeparationRunning,
  } from "$lib/process";
  import ProcessCard from "../components/process/ProcessCard.svelte";
  import EmptyState from "../components/process/EmptyState.svelte";

  // FR-01 — payload { ids, model } 추출. goBack 재진입 시 payload가 null이므로
  // 마지막 배치(store layer 보존)로 fallback → 진행률 그대로 표시 (SC-10).
  let batchIds = $state<string[]>([]);
  let selectedId = $state<string | null>(null);

  // FR-01 — queueStore가 SoT. 배치 ids에 해당하는 항목만 카드로 표시.
  let items = $derived($queueStore.filter((it) => batchIds.includes(it.id)));

  onMount(() => {
    const payload = get(pagePayload);
    if (
      payload?.kind === "process" &&
      payload.ids.length > 0 &&
      !isSeparationRunning()
    ) {
      batchIds = payload.ids;
      // 순차 처리 시작 (NFR Concurrency — 내부 running 가드가 2차 방어)
      void startSeparationBatch(payload.ids, payload.model);
    } else {
      // goBack 재진입(payload=null) 또는 배치 진행 중 → 진행 중 배치를 그대로 표시
      const last = get(processBatch);
      batchIds = last?.ids ?? [];
    }
    window.addEventListener("keydown", handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleKeydown);
  });

  // ─── 선택 (Phase 1 단순화 — 단일 클릭 토글) ───────────────────────────────

  function handleSelect(id: string): void {
    selectedId = selectedId === id ? null : id;
  }

  // ─── 취소 (FR-08 — queue-page cancel_queue_item 재사용) ───────────────────

  function cancelItem(id: string): void {
    const item = items.find((it) => it.id === id);
    if (!item || item.status !== "in-progress") return;
    // kill → separate_audio가 에러로 반환 → loop가 status="error" 기록 후 다음 항목 계속
    void cancelQueueItem(id).catch((err) => {
      console.warn("cancel_queue_item failed (non-fatal):", err);
    });
  }

  function openItem(id: string): void {
    navigateTo("player", { kind: "player", itemId: id });
  }

  // ─── 단축키 (FR-13 / SHORTCUTS.md) ─────────────────────────────────────────

  function isTypingTarget(target: EventTarget | null): boolean {
    if (!(target instanceof HTMLElement)) return false;
    const tag = target.tagName;
    return tag === "INPUT" || tag === "TEXTAREA" || target.isContentEditable;
  }

  function handleKeydown(e: KeyboardEvent): void {
    if (isTypingTarget(e.target)) return;

    if (e.key === "Escape") {
      e.preventDefault();
      navigateTo("queue");
    } else if (e.key === "Delete") {
      if (selectedId) {
        e.preventDefault();
        cancelItem(selectedId);
      }
    }
  }
</script>

<div class="flex h-full flex-col gap-4 p-6">
  {#if items.length === 0}
    <!-- FR-12 / SC-16 — 실수 진입 방어 -->
    <div class="flex-1" in:fade={{ duration: 200 }}>
      <EmptyState />
    </div>
  {:else}
    <div class="flex items-center justify-between">
      <h2 class="text-xl font-bold text-text">음원 분리</h2>
      <span class="text-xs text-muted">
        완료된 곡은 자동으로 열려요 · 나머지는 순서대로 계속 처리돼요
      </span>
    </div>

    <div class="flex-1 overflow-y-auto">
      <div class="flex flex-col gap-2">
        {#each items as item (item.id)}
          <div in:fade={{ duration: 150 }}>
            <ProcessCard
              {item}
              selected={selectedId === item.id}
              onSelect={() => handleSelect(item.id)}
              onCancel={() => cancelItem(item.id)}
              onOpen={() => openItem(item.id)}
            />
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>
