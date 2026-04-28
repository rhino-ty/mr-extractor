<script lang="ts">
  // Design Ref: §5.3 — EnvItemRow: 아이콘 + 라벨 + 버전. Plan FR-06: 한국어 라벨만 노출.
  import type { EnvItem } from "$lib/types";

  interface Props {
    item: EnvItem;
  }

  let { item }: Props = $props();

  const ICON: Record<EnvItem["status"], string> = {
    ready: "✅",
    missing: "○",
    installing: "⏳",
    error: "❌",
  };

  const COLOR: Record<EnvItem["status"], string> = {
    ready: "text-success",
    missing: "text-muted",
    installing: "text-accent",
    error: "text-danger",
  };
</script>

<div class="flex items-center gap-3 py-1.5">
  <span class="{COLOR[item.status]} w-5 text-center">{ICON[item.status]}</span>
  <span class="flex-1 text-sm">{item.label}</span>
  {#if item.version}
    <span class="text-xs text-muted">v{item.version}</span>
  {/if}
</div>
