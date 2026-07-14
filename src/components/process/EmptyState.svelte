<script lang="ts">
  // Design Ref: ProcessPage Plan FR-12 — 실수 진입 방어 (payload 없이 직접 진입)
  import { navigateTo } from "$lib/stores";
</script>

<div class="flex flex-col items-center justify-center gap-3 py-16 text-center">
  <div class="text-5xl" aria-hidden="true">🎧</div>
  <p class="text-lg font-semibold text-text">처리할 항목이 없어요</p>
  <p class="text-sm text-muted">큐에서 항목을 선택하고 분리를 시작해 주세요</p>
  <button
    type="button"
    class="mt-2 rounded-lg bg-accent px-4 py-2 text-sm font-medium text-white transition-colors duration-200 hover:bg-accent/80"
    onclick={() => navigateTo("queue")}
  >
    큐로 돌아가기
  </button>
</div>
