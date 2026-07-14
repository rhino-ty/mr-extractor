<script lang="ts">
  // Design Ref: SETTINGS.md — 환경 상태 / 저장 공간 / 알림 / 내보내기 포맷 + 단축키 목록.
  // 환경 상태는 setup::check_environment(EnvStatus) 재사용 (SETTINGS.md Current Rules).
  // 저장 정책(keep_all 등)은 적용 메커니즘(app-lifecycle)과 함께 후속 도입 — 죽은 설정 지양.

  import { onDestroy, onMount } from "svelte";
  import { fade } from "svelte/transition";
  import { open as shellOpen } from "@tauri-apps/plugin-shell";
  import { goBack, isProcessing, pushToast } from "$lib/stores";
  import {
    checkEnvironment,
    clearQueueTmp,
    storageStats,
  } from "$lib/commands";
  import { appSettings, updateSettings } from "$lib/settings";
  import type { EnvStatus, ExportFormat, StorageStats } from "$lib/types";

  let env = $state<EnvStatus | null>(null);
  let envLoading = $state(true);
  let stats = $state<StorageStats | null>(null);
  let confirmClear = $state(false);
  let clearing = $state(false);

  let settings = $derived($appSettings);
  let processing = $derived($isProcessing);

  const FORMAT_OPTIONS: Array<{ value: ExportFormat; label: string }> = [
    { value: "wav", label: "WAV" },
    { value: "mp3", label: "MP3 320k" },
    { value: "flac", label: "FLAC" },
  ];

  // SHORTCUTS.md 표 — 설정 페이지 단축키 목록 섹션
  const SHORTCUTS: Array<[string, string]> = [
    ["Space", "재생/일시정지 (플레이어)"],
    ["← / →", "5초 이동 · Shift 30초 (플레이어)"],
    ["↑ / ↓", "마스터 볼륨 ±5 (플레이어)"],
    ["M", "전체 뮤트 토글 (플레이어)"],
    ["Ctrl + O", "파일 열기 (전역)"],
    ["Ctrl + H", "히스토리 토글 (전역)"],
    ["Ctrl + ,", "설정 열기 (전역)"],
    ["Ctrl + A", "전체 선택 (큐/히스토리)"],
    ["Delete", "선택 항목 삭제/취소 (큐/처리/히스토리)"],
    ["Escape", "뒤로/선택 해제 (전역)"],
  ];

  async function refreshEnv(): Promise<void> {
    envLoading = true;
    try {
      env = await checkEnvironment();
    } catch (err) {
      pushToast(typeof err === "string" ? err : String(err), "error");
    } finally {
      envLoading = false;
    }
  }

  async function refreshStats(): Promise<void> {
    try {
      stats = await storageStats();
    } catch {
      // 실측 실패는 조용히 — 섹션에 "확인 불가" 표시
      stats = null;
    }
  }

  onMount(() => {
    void refreshEnv();
    void refreshStats();
    window.addEventListener("keydown", handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleKeydown);
  });

  function handleKeydown(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      if (confirmClear) {
        confirmClear = false;
      } else {
        goBack();
      }
    }
  }

  async function runClearTmp(): Promise<void> {
    confirmClear = false;
    clearing = true;
    try {
      const freed = await clearQueueTmp();
      pushToast(`임시 파일을 정리했어요. (${formatMb(freed)} 확보)`, "success");
      await refreshStats();
    } catch (err) {
      pushToast(typeof err === "string" ? err : String(err), "error");
    } finally {
      clearing = false;
    }
  }

  async function openOutputFolder(): Promise<void> {
    if (!stats) return;
    try {
      await shellOpen(stats.outputDir);
    } catch {
      pushToast("폴더를 열 수 없어요. 아직 내보낸 파일이 없을 수 있어요.", "warn");
    }
  }

  function formatMb(mb: number): string {
    return mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${mb} MB`;
  }

  const STATUS_ICONS: Record<string, string> = {
    ready: "✅",
    missing: "❌",
    installing: "⏳",
    error: "❌",
  };
</script>

<div class="flex h-full flex-col gap-4 overflow-y-auto p-6">
  <h2 class="text-xl font-bold text-text">설정</h2>

  <!-- 환경 상태 (SETTINGS.md 패널) -->
  <section class="rounded-xl border border-border bg-surface p-4">
    <div class="mb-3 flex items-center justify-between">
      <h3 class="text-sm font-semibold text-text">환경 상태</h3>
      <button
        type="button"
        class="rounded-md border border-border px-2 py-1 text-xs text-muted transition-colors duration-200 hover:border-accent/40 hover:text-text disabled:opacity-50"
        disabled={envLoading}
        onclick={() => void refreshEnv()}
      >
        🔄 새로고침
      </button>
    </div>
    {#if envLoading}
      <div class="flex items-center gap-2 py-2 text-sm text-muted" in:fade={{ duration: 150 }}>
        <div
          class="h-4 w-4 animate-spin rounded-full border-2 border-border border-t-accent"
          aria-hidden="true"
        ></div>
        확인 중...
      </div>
    {:else if env}
      <div class="flex flex-col gap-1.5">
        {#each env.items as item (item.label)}
          <div class="flex items-center justify-between text-sm">
            <span class="text-text">{item.label}</span>
            <span class="flex items-center gap-2 text-xs text-muted">
              {#if item.version}<span>{item.version}</span>{/if}
              <span>{STATUS_ICONS[item.status] ?? "○"}</span>
            </span>
          </div>
        {/each}
      </div>
    {:else}
      <p class="text-sm text-muted">환경 정보를 확인할 수 없어요.</p>
    {/if}
  </section>

  <!-- 저장 공간 (SETTINGS.md) -->
  <section class="rounded-xl border border-border bg-surface p-4">
    <h3 class="mb-3 text-sm font-semibold text-text">저장 공간</h3>
    {#if stats}
      <div class="flex flex-col gap-2 text-sm">
        <div class="flex items-center justify-between">
          <span class="text-muted">출력 폴더</span>
          <span class="flex items-center gap-2">
            <span class="max-w-72 truncate text-xs text-text" title={stats.outputDir}>
              {stats.outputDir}
            </span>
            <button
              type="button"
              class="rounded-md border border-border px-2 py-1 text-xs text-muted transition-colors duration-200 hover:border-accent/40 hover:text-text"
              onclick={() => void openOutputFolder()}
            >
              📁 폴더 열기
            </button>
          </span>
        </div>
        <div class="flex items-center justify-between">
          <span class="text-muted">내보낸 파일</span>
          <span class="text-xs text-text">{formatMb(stats.outputMb)}</span>
        </div>
        <div class="flex items-center justify-between">
          <span class="text-muted">임시 파일 (다운로드 + 분리 스템)</span>
          <span class="flex items-center gap-2">
            <span class="text-xs text-text">{formatMb(stats.queueTmpMb)}</span>
            <button
              type="button"
              class="rounded-md border border-border px-2 py-1 text-xs text-muted transition-colors duration-200 hover:border-warn/60 hover:text-warn disabled:cursor-not-allowed disabled:opacity-40"
              disabled={clearing || processing || stats.queueTmpMb === 0}
              title={processing
                ? "처리 중에는 정리할 수 없어요"
                : "임시 파일을 모두 삭제해요 (분리 스템 포함)"}
              onclick={() => (confirmClear = true)}
            >
              🧹 임시 파일 정리
            </button>
          </span>
        </div>
        <div class="flex items-center justify-between">
          <span class="text-muted">AI 모델 캐시</span>
          <span class="text-xs text-text">{formatMb(stats.modelCacheMb)}</span>
        </div>
        <div class="flex items-center justify-between border-t border-border pt-2">
          <span class="text-muted">앱 데이터 전체</span>
          <span class="text-xs text-text">{formatMb(stats.appDataMb)}</span>
        </div>
      </div>
    {:else}
      <p class="text-sm text-muted">저장 공간 정보를 확인할 수 없어요.</p>
    {/if}
  </section>

  <!-- 알림 (SETTINGS.md) -->
  <section class="rounded-xl border border-border bg-surface p-4">
    <h3 class="mb-3 text-sm font-semibold text-text">알림</h3>
    <label class="flex cursor-pointer items-center justify-between text-sm">
      <span class="text-muted">처리 완료 시 시스템 알림</span>
      <input
        type="checkbox"
        class="h-4 w-4 accent-accent"
        checked={settings.notifyEnabled}
        onchange={(e) => updateSettings({ notifyEnabled: e.currentTarget.checked })}
      />
    </label>
  </section>

  <!-- 내보내기 (SETTINGS.md) -->
  <section class="rounded-xl border border-border bg-surface p-4">
    <h3 class="mb-3 text-sm font-semibold text-text">내보내기</h3>
    <div class="flex items-center justify-between text-sm">
      <span class="text-muted">기본 포맷</span>
      <div class="flex gap-1.5">
        {#each FORMAT_OPTIONS as f (f.value)}
          <button
            type="button"
            class="rounded-lg border px-3 py-1.5 text-xs transition-colors duration-200 {settings.defaultFormat ===
            f.value
              ? 'border-accent bg-accent/20 text-text'
              : 'border-border text-muted hover:border-accent/40 hover:text-text'}"
            onclick={() => updateSettings({ defaultFormat: f.value })}
          >
            {f.label}
          </button>
        {/each}
      </div>
    </div>
  </section>

  <!-- 단축키 (SHORTCUTS.md — 설정 페이지 목록 섹션) -->
  <section class="rounded-xl border border-border bg-surface p-4">
    <h3 class="mb-3 text-sm font-semibold text-text">단축키</h3>
    <div class="grid grid-cols-1 gap-1.5 md:grid-cols-2">
      {#each SHORTCUTS as [key, desc] (key)}
        <div class="flex items-center justify-between gap-3 text-sm">
          <kbd
            class="rounded-md border border-border bg-bg px-2 py-0.5 font-mono text-[11px] text-text"
          >
            {key}
          </kbd>
          <span class="flex-1 text-right text-xs text-muted">{desc}</span>
        </div>
      {/each}
    </div>
  </section>
</div>

<!-- 임시 파일 정리 확인 다이얼로그 -->
{#if confirmClear}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-bg/70"
    transition:fade={{ duration: 150 }}
  >
    <div class="w-96 rounded-xl border border-border bg-surface p-5 shadow-xl">
      <h3 class="text-sm font-semibold text-text">임시 파일 정리</h3>
      <p class="mt-2 text-sm text-muted">
        다운로드한 오디오와 분리된 스템 파일이 모두 삭제돼요. 플레이어에서 다시
        열려면 재처리가 필요해요. 내보낸 반주 파일은 유지돼요.
      </p>
      <div class="mt-5 flex justify-end gap-2">
        <button
          type="button"
          class="rounded-lg border border-border px-4 py-2 text-sm text-muted transition-colors duration-200 hover:text-text"
          onclick={() => (confirmClear = false)}
        >
          취소
        </button>
        <button
          type="button"
          class="rounded-lg bg-warn px-4 py-2 text-sm font-medium text-bg transition-colors duration-200 hover:bg-warn/80"
          onclick={() => void runClearTmp()}
        >
          🧹 정리
        </button>
      </div>
    </div>
  </div>
{/if}
