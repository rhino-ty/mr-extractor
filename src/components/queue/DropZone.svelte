<script lang="ts">
  // Design Ref: §5.3 — 파일 드래그 + drag-over visual + Ctrl+O 파일 다이얼로그 fallback
  // Plan FR-02 — 다중 파일 동시 처리, < 100ms 시각 피드백

  import { onMount, onDestroy } from "svelte";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { openFileDialog, processDroppedPaths } from "$lib/queue";

  let isDragOver = $state(false);
  let unlistenDragDrop: (() => void) | null = null;

  onMount(async () => {
    const webview = getCurrentWebview();
    const unsub = await webview.onDragDropEvent((event) => {
      if (event.payload.type === "enter" || event.payload.type === "over") {
        isDragOver = true;
      } else if (event.payload.type === "leave") {
        isDragOver = false;
      } else if (event.payload.type === "drop") {
        isDragOver = false;
        processDroppedPaths(event.payload.paths);
      }
    });
    unlistenDragDrop = unsub;
  });

  onDestroy(() => {
    unlistenDragDrop?.();
  });
</script>

<button
  type="button"
  class="flex w-full flex-col items-center justify-center gap-2 rounded-xl border-2 border-dashed px-6 py-8 text-center transition-all duration-100 {isDragOver
    ? 'border-accent bg-accent/10'
    : 'border-border bg-surface hover:border-accent/50 hover:bg-surface/70'}"
  onclick={openFileDialog}
  aria-label="파일 추가"
>
  <span class="text-2xl">📂</span>
  <span class="text-sm font-medium text-text">파일을 끌어다 놓기</span>
  <span class="text-xs text-muted">또는 클릭 / Ctrl+O</span>
</button>
