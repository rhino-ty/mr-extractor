<script lang="ts">
  // Design Ref: §5.1 -- 공통 헤더 + 페이지 라우팅 셸
  import { page, navigateTo, goBack, pageHistory } from "$lib/stores";
  import { fade } from "svelte/transition";
  import { derived } from "svelte/store";

  import SetupPage from "./pages/SetupPage.svelte";
  import QueuePage from "./pages/QueuePage.svelte";
  import ProcessPage from "./pages/ProcessPage.svelte";
  import PlayerPage from "./pages/PlayerPage.svelte";
  import HistoryPage from "./pages/HistoryPage.svelte";
  import SettingsPage from "./pages/SettingsPage.svelte";

  // 뒤로 버튼을 숨길 페이지 (메인 페이지들)
  const mainPages = new Set(["setup", "queue"]);

  const showBack = derived([page, pageHistory], ([$p, $h]) => {
    return !mainPages.has($p) && $h.length > 0;
  });
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
