<script lang="ts">
  // Design Ref: §5.1 -- 공통 헤더 + 페이지 라우팅 셸
  import { onDestroy, onMount } from "svelte";
  import { get } from "svelte/store";
  import { page, navigateTo, goBack, pageHistory, hydrateQueueStore } from "$lib/stores";
  import { openFileDialog } from "$lib/queue";
  import { hydrateSettings } from "$lib/settings";
  import { fade } from "svelte/transition";
  import { derived } from "svelte/store";

  import SetupPage from "./pages/SetupPage.svelte";
  import QueuePage from "./pages/QueuePage.svelte";
  import ProcessPage from "./pages/ProcessPage.svelte";
  import PlayerPage from "./pages/PlayerPage.svelte";
  import HistoryPage from "./pages/HistoryPage.svelte";
  import SettingsPage from "./pages/SettingsPage.svelte";
  import Toast from "./components/common/Toast.svelte";

  // 뒤로 버튼을 숨길 페이지 (메인 페이지들)
  const mainPages = new Set(["setup", "queue"]);

  // UX M5 — 헤더에 현재 페이지 표시 (깊은 페이지에서 위치 인지)
  const PAGE_LABELS: Record<string, string> = {
    process: "음원 분리",
    player: "플레이어",
    history: "히스토리",
    settings: "설정",
  };

  const showBack = derived([page, pageHistory], ([$p, $h]) => {
    return !mainPages.has($p) && $h.length > 0;
  });

  // Plan SC-7 — 앱 시작 시 큐 hydrate (파일 mismatch 자동 제거) + 설정 hydrate (v1.2)
  onMount(() => {
    void hydrateQueueStore();
    void hydrateSettings();
    window.addEventListener("keydown", handleGlobalKeydown);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleGlobalKeydown);
  });

  // Iterate 1 (I-2) / Plan FR-21 / SC-18 — Ctrl+O 전역 단축키.
  // 임의 페이지에서 Ctrl+O → queue 페이지로 이동 + 파일 다이얼로그 오픈.
  // input/textarea/contenteditable 입력 중에는 skip.
  // Windows 전용 프로젝트 → Meta(Cmd) 매칭 불필요.
  function isTypingTarget(target: EventTarget | null): boolean {
    if (!(target instanceof HTMLElement)) return false;
    const tag = target.tagName;
    return tag === "INPUT" || tag === "TEXTAREA" || target.isContentEditable;
  }

  function handleGlobalKeydown(e: KeyboardEvent): void {
    if (!(e.ctrlKey && !e.metaKey && !e.altKey && !e.shiftKey)) return;
    if (isTypingTarget(e.target)) return;

    const key = e.key.toLowerCase();
    if (key === "o") {
      e.preventDefault();
      // QueuePage가 자체 Ctrl+O를 처리하지 않도록 — 전역에서 처리. (QueuePage 로컬 핸들러 제거됨)
      if (get(page) !== "queue") {
        navigateTo("queue");
      }
      void openFileDialog();
    } else if (key === "h") {
      // SHORTCUTS.md — Ctrl+H 히스토리 토글
      e.preventDefault();
      if (get(page) === "history") {
        goBack();
      } else {
        navigateTo("history");
      }
    } else if (e.key === ",") {
      // SHORTCUTS.md — Ctrl+, 설정
      e.preventDefault();
      if (get(page) !== "settings") {
        navigateTo("settings");
      }
    }
  }
</script>

<div class="flex h-full flex-col bg-bg">
  <!-- Header -->
  <header
    class="flex items-center justify-between border-b border-border px-4 py-3"
  >
    <div class="flex items-center gap-3">
      {#if $showBack}
        <button
          onclick={() => goBack()}
          class="rounded-lg px-2 py-1 text-sm text-muted transition-colors duration-200 hover:bg-surface hover:text-text"
        >
          &larr; 뒤로
        </button>
      {/if}
      <h1 class="text-lg font-bold text-text">MR Extractor</h1>
      {#if PAGE_LABELS[$page]}
        <span class="text-sm text-muted">· {PAGE_LABELS[$page]}</span>
      {/if}
    </div>
    <div class="flex items-center gap-2">
      <button
        onclick={() => navigateTo("history")}
        class="rounded-lg px-3 py-1.5 text-sm text-muted transition-colors duration-200 hover:bg-surface hover:text-text"
        title="히스토리 (Ctrl+H)"
      >
        히스토리
      </button>
      <button
        onclick={() => navigateTo("settings")}
        class="rounded-lg px-3 py-1.5 text-sm text-muted transition-colors duration-200 hover:bg-surface hover:text-text"
        title="설정 (Ctrl+,)"
      >
        설정
      </button>
    </div>
  </header>

  <!-- Page Content -->
  <main class="flex-1 overflow-auto">
    {#key $page}
      <div
        class="h-full"
        in:fade={{ duration: 200 }}
      >
        {#if $page === "setup"}
          <SetupPage />
        {:else if $page === "queue"}
          <QueuePage />
        {:else if $page === "process"}
          <ProcessPage />
        {:else if $page === "player"}
          <PlayerPage />
        {:else if $page === "history"}
          <HistoryPage />
        {:else if $page === "settings"}
          <SettingsPage />
        {/if}
      </div>
    {/key}
  </main>
</div>

<!-- 전역 Toast (모든 페이지 공유) -->
<Toast />
