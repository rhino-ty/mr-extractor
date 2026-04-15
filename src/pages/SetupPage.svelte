<script lang="ts">
  import { navigateTo } from "$lib/stores";
  import { onMount } from "svelte";

  // Plan SC: 2회차 이후 모두 ready → 1초 후 자동 진입
  onMount(() => {
    setTimeout(() => navigateTo("queue"), 1500);
  });
</script>

<div class="flex h-full flex-col items-center justify-center gap-4">
  <div class="text-4xl">🎵</div>
  <h2 class="text-xl font-bold">MR Extractor</h2>
  <p class="text-sm text-muted">앱을 사용할 준비를 하고 있어요...</p>
  <div class="mt-4 h-1 w-48 overflow-hidden rounded-full bg-surface">
    <div class="h-full animate-pulse rounded-full bg-accent" style="width: 60%"></div>
  </div>
  <p class="text-xs text-text-secondary">처음 실행 시 한 번만 설치됩니다.</p>
</div>
