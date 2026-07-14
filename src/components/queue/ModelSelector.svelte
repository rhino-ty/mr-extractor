<script lang="ts">
  // Design Ref: MODEL_SELECTOR.md — 모델 드롭다운 + [?] 도움말 패널 + on-demand 다운로드.
  // htdemucs_6s는 표시하되 비활성: 6s의 other.wav에는 기타/피아노가 빠져 있어
  // 현재 4-스템 믹서/내보내기로는 소리가 누락됨 → 6-스템 믹서 도입 시 활성화.

  import { onDestroy, onMount } from "svelte";
  import { get } from "svelte/store";
  import { fade, slide } from "svelte/transition";
  import { selectedModel, pushToast } from "$lib/stores";
  import { downloadModelByName, listModels } from "$lib/commands";
  import type { ModelId, ModelInfo } from "$lib/types";

  interface ModelMeta {
    id: ModelId;
    name: string;
    tooltip: string;
    disabledReason?: string;
  }

  const MODELS: ModelMeta[] = [
    {
      id: "htdemucs_ft",
      name: "고품질 (권장)",
      tooltip: "최고 분리 품질. 전용 모델 4개를 돌려 처리 시간이 약 4배 걸려요.",
    },
    {
      id: "htdemucs",
      name: "빠른 분리",
      tooltip: "품질은 조금 낮지만 훨씬 빨라요. 빠른 미리듣기용.",
    },
    {
      id: "htdemucs_6s",
      name: "6트랙 분리",
      tooltip: "기타·피아노까지 따로 분리 (실험적)",
      disabledReason: "기타/피아노 트랙 믹서가 준비되면 열려요",
    },
  ];

  let installed = $state<Record<string, boolean>>({});
  let helpOpen = $state(false);
  let downloading = $state<ModelId | null>(null);
  let downloadPercent = $state(0);
  // 다운로드 확인 다이얼로그 대상 (MODEL_SELECTOR.md 플로우 3단계)
  let confirmTarget = $state<ModelMeta | null>(null);

  let current = $derived($selectedModel);

  onMount(() => {
    void refreshInstalled();
    window.addEventListener("keydown", handleKeydown, true);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleKeydown, true);
  });

  // UX M2 — SHORTCUTS.md "Escape = 모달/도움말 닫기" (capture로 QueuePage보다 먼저)
  function handleKeydown(e: KeyboardEvent): void {
    if (e.key !== "Escape") return;
    if (confirmTarget) {
      e.preventDefault();
      e.stopPropagation();
      confirmTarget = null;
    } else if (helpOpen) {
      e.preventDefault();
      e.stopPropagation();
      helpOpen = false;
    }
  }

  async function refreshInstalled(): Promise<void> {
    try {
      const list: ModelInfo[] = await listModels();
      installed = Object.fromEntries(list.map((m) => [m.id, m.installed]));
    } catch {
      // 조회 실패 시 기본 모델만 신뢰 (setup-page 보장)
      installed = { htdemucs_ft: true };
    }
  }

  function pick(meta: ModelMeta): void {
    if (meta.disabledReason || downloading) return;
    if (meta.id === get(selectedModel)) return;
    if (installed[meta.id]) {
      selectedModel.set(meta.id);
      return;
    }
    confirmTarget = meta; // 미설치 → 다운로드 확인
  }

  async function confirmDownload(): Promise<void> {
    const meta = confirmTarget;
    confirmTarget = null;
    if (!meta) return;
    downloading = meta.id;
    downloadPercent = 0;
    try {
      await downloadModelByName(meta.id, (p) => {
        downloadPercent = p.percent;
      });
      installed = { ...installed, [meta.id]: true };
      selectedModel.set(meta.id);
      pushToast(`${meta.name} 모델이 준비됐어요.`, "success");
    } catch (err) {
      pushToast(typeof err === "string" ? err : String(err), "error");
    } finally {
      downloading = null;
    }
  }
</script>

<div class="flex flex-col gap-2">
  <div class="flex items-center gap-2">
    <span class="text-xs text-muted">모델</span>
    <div class="flex gap-1.5">
      {#each MODELS as meta (meta.id)}
        <button
          type="button"
          class="rounded-lg border px-3 py-1.5 text-xs transition-colors duration-200 disabled:cursor-not-allowed disabled:opacity-40 {current ===
          meta.id
            ? 'border-accent bg-accent/20 text-text'
            : 'border-border text-muted hover:border-accent/40 hover:text-text'}"
          title={meta.disabledReason ?? meta.tooltip}
          disabled={!!meta.disabledReason || downloading !== null}
          onclick={() => pick(meta)}
        >
          {meta.name}
          {#if !meta.disabledReason && installed[meta.id] === false}
            <span class="text-muted">⬇</span>
          {/if}
        </button>
      {/each}
    </div>
    <button
      type="button"
      class="rounded-full border border-border px-2 py-0.5 text-xs text-muted transition-colors duration-200 hover:border-accent/40 hover:text-text"
      title="모델 선택 가이드"
      aria-expanded={helpOpen}
      onclick={() => (helpOpen = !helpOpen)}
    >
      ?
    </button>

    {#if downloading}
      <div class="flex flex-1 items-center gap-2" in:fade={{ duration: 150 }}>
        <div class="h-1 max-w-40 flex-1 overflow-hidden rounded-full bg-bg">
          <div
            class="h-full bg-accent transition-all duration-200"
            style="width: {downloadPercent}%"
          ></div>
        </div>
        <span class="text-xs text-muted">모델 받는 중... {downloadPercent}%</span>
      </div>
    {/if}
  </div>

  <!-- [?] 도움말 패널 (MODEL_SELECTOR.md) -->
  {#if helpOpen}
    <div
      class="flex flex-col gap-2 rounded-xl border border-border bg-surface p-4 text-sm"
      transition:slide={{ duration: 200 }}
    >
      <div class="flex items-center justify-between">
        <span class="font-semibold text-text">모델 선택 가이드</span>
        <button
          type="button"
          class="text-muted transition-colors duration-200 hover:text-text"
          aria-label="닫기"
          onclick={() => (helpOpen = false)}
        >
          ✕
        </button>
      </div>
      <p class="text-muted">
        <span class="text-text">🏆 고품질 (권장)</span> — 현재 가장 높은 분리 품질.
        대부분은 이걸 쓰세요. 단, 처리 시간이 빠른 모델의 약 4배예요.
      </p>
      <p class="text-muted">
        <span class="text-text">⚡ 빠른 분리</span> — 품질을 조금 양보하고 속도를
        얻어요. 빠른 미리듣기용으로 적합해요.
      </p>
      <p class="text-muted">
        <span class="text-text">🎸 6트랙 분리</span> — 기타·피아노까지 따로 분리하는
        실험적 모델이에요. 전용 믹서가 준비되면 사용할 수 있어요.
      </p>
    </div>
  {/if}
</div>

<!-- 다운로드 확인 다이얼로그 (MODEL_SELECTOR.md 플로우) -->
{#if confirmTarget}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-bg/70"
    transition:fade={{ duration: 150 }}
  >
    <div class="w-96 rounded-xl border border-border bg-surface p-5 shadow-xl" role="dialog" aria-modal="true">
      <h3 class="text-sm font-semibold text-text">
        {confirmTarget.name} 모델 다운로드
      </h3>
      <p class="mt-2 text-sm text-muted">
        이 모델은 처음 사용할 때 한 번만 다운로드해요. 인터넷 연결이 필요해요.
      </p>
      <div class="mt-5 flex justify-end gap-2">
        <button
          type="button"
          class="rounded-lg border border-border px-4 py-2 text-sm text-muted transition-colors duration-200 hover:text-text"
          onclick={() => (confirmTarget = null)}
        >
          취소
        </button>
        <button
          type="button"
          class="rounded-lg bg-accent px-4 py-2 text-sm font-medium text-white transition-colors duration-200 hover:bg-accent/80"
          onclick={() => void confirmDownload()}
        >
          ⬇ 다운로드
        </button>
      </div>
    </div>
  </div>
{/if}
