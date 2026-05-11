<script lang="ts">
  // Design Ref: §5.2 / §5.3 — 우상단 stack, 3초 후 dismiss, kind별 색
  // Plan §10.2 — 자체 구현 (외부 라이브러리 0건)

  import { fade, fly } from "svelte/transition";
  import { toastStore } from "$lib/stores";
  import type { Toast, ToastKind } from "$lib/types";

  const TOAST_STYLES: Record<ToastKind, { border: string; bg: string; icon: string }> = {
    info: { border: "border-accent/60", bg: "bg-accent/10", icon: "ℹ" },
    success: { border: "border-success/60", bg: "bg-success/10", icon: "✓" },
    warn: { border: "border-warn/60", bg: "bg-warn/10", icon: "⚠" },
    error: { border: "border-danger/60", bg: "bg-danger/10", icon: "✕" },
  };

  function dismiss(id: string) {
    toastStore.update((arr) => arr.filter((t) => t.id !== id));
  }
</script>

<div
  class="pointer-events-none fixed top-4 right-4 z-50 flex flex-col gap-2"
  aria-live="polite"
  aria-atomic="true"
>
  {#each [...$toastStore].reverse() as toast (toast.id)}
    {@const style = TOAST_STYLES[toast.kind]}
    <div
      class="pointer-events-auto flex min-w-[260px] max-w-sm items-start gap-3 rounded-lg border bg-surface px-4 py-3 text-sm text-text shadow-lg {style.border} {style.bg}"
      in:fly={{ x: 20, duration: 200 }}
      out:fade={{ duration: 150 }}
      role="status"
    >
      <span class="text-base leading-5">{style.icon}</span>
      <span class="flex-1 leading-5 break-words">{toast.message}</span>
      <button
        type="button"
        class="text-muted hover:text-text transition-colors"
        aria-label="닫기"
        onclick={() => dismiss(toast.id)}
      >
        ✕
      </button>
    </div>
  {/each}
</div>
