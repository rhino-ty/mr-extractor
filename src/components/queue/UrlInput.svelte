<script lang="ts">
  // Design Ref: §5.3 — URL 입력 + Enter + invalid inline error
  // Plan SC-1 — 1초 내 placeholder 카드 등장 (라벨 = URL, 길이 미상)
  // Plan FR-14 — youtube/youtu.be/m./music. 정규화

  import { addToQueue, normalizeUrl } from "$lib/queue";

  let raw = $state("");
  let errorMessage = $state<string | null>(null);

  function submit(): void {
    const trimmed = raw.trim();
    if (!trimmed) {
      errorMessage = null;
      return;
    }
    const normalized = normalizeUrl(trimmed);
    if (!normalized) {
      errorMessage = "올바른 YouTube URL이 아니에요";
      return;
    }
    const added = addToQueue({
      sourceType: "youtube",
      source: normalized,
      label: "다운로드 준비 중...",
    });
    if (added) {
      raw = "";
      errorMessage = null;
    }
  }

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === "Enter") {
      e.preventDefault();
      submit();
    }
  }

  function onInput(): void {
    if (errorMessage) errorMessage = null;
  }
</script>

<div class="flex w-full flex-col gap-1">
  <div class="flex items-center gap-2 rounded-xl border bg-surface px-4 py-3 transition-colors {errorMessage ? 'border-danger' : 'border-border focus-within:border-accent'}">
    <span class="text-base">🔗</span>
    <input
      type="text"
      bind:value={raw}
      onkeydown={onKeydown}
      oninput={onInput}
      placeholder="YouTube URL을 붙여넣고 Enter"
      class="flex-1 bg-transparent text-sm text-text placeholder:text-muted focus:outline-none"
      aria-label="YouTube URL"
      aria-invalid={errorMessage !== null}
    />
    <button
      type="button"
      onclick={submit}
      class="rounded-md bg-accent px-3 py-1.5 text-xs font-medium text-white transition-colors hover:bg-accent/80 disabled:opacity-50"
      disabled={!raw.trim()}
    >
      추가 ➤
    </button>
  </div>
  {#if errorMessage}
    <span class="px-1 text-xs text-danger">{errorMessage}</span>
  {/if}
</div>
