<script lang="ts">
  import { navigateTo } from "$lib/stores";
</script>

<div class="flex h-full flex-col items-center justify-center gap-4">
  <div class="text-4xl">📋</div>
  <h2 class="text-xl font-bold">Queue</h2>
  <p class="text-sm text-muted">URL을 입력하거나 파일을 드래그하세요</p>
  <button
    onclick={() => navigateTo("process")}
    class="mt-4 rounded-lg bg-accent px-6 py-2 text-sm font-medium text-white transition-colors duration-200 hover:bg-accent/80"
  >
    분리 시작 (테스트)
  </button>
</div>
