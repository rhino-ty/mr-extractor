<script lang="ts">
  import { navigateTo } from "$lib/stores";
</script>

<div class="flex h-full flex-col items-center justify-center gap-4">
  <div class="text-4xl">⏳</div>
  <h2 class="text-xl font-bold">Processing</h2>
  <p class="text-sm text-muted">분리 진행 중...</p>
  <button
    onclick={() => navigateTo("player")}
    class="mt-4 rounded-lg bg-success px-6 py-2 text-sm font-medium text-bg transition-colors duration-200 hover:bg-success/80"
  >
    완료 (테스트)
  </button>
</div>
