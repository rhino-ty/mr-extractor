<script lang="ts">
  // Design Ref: §5.1 — 6-state 머신 (detecting/installing/ready/error/no-internet/disk-full)
  // Plan SC-2: 2회차 이후 모두 ready → 1초 후 자동 QueuePage 진입
  // Plan SC-8: 기술 용어 노출 금지 ("Python"/"demucs"/"pip"/"torch" 사용 X)
  //
  // Phase 1 구현: detecting + ready 실구현. 나머지는 상태 표현 + 기본 UI만 (logic은 Phase 2/3).

  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import { navigateTo } from "$lib/stores";
  import {
    cancelInstall,
    checkDiskSpace,
    checkEnvironment,
    checkInternet,
    installDependencies,
  } from "$lib/commands";
  import { translateToFriendlyMessage } from "$lib/errorMessages";
  import type { EnvItem, InstallProgress, SetupPageState } from "$lib/types";
  import EnvItemRow from "../components/setup/EnvItemRow.svelte";

  let pageState: SetupPageState = $state({ kind: "detecting" });
  let detailOpen = $state(false);
  let canceling = $state(false);

  // Design §5.1 — installing 진입 시 "현재 진행 중인 항목"을 ⏳로 표시.
  // Plan FR-06: 라벨은 한국어 별칭만 사용.
  function applyPhaseToItems(items: EnvItem[], progress: InstallProgress): EnvItem[] {
    const PHASE_LABEL: Record<InstallProgress["phase"], string> = {
      extract_python: "실행 환경",
      create_venv: "실행 환경",
      install_torch: "음원 분리 엔진",
      install_demucs: "음원 분리 엔진",
      download_model: "AI 모델",
    };
    const activeLabel = PHASE_LABEL[progress.phase];
    return items.map((it) => {
      if (it.label === activeLabel && it.status !== "ready") {
        return { ...it, status: "installing" as const };
      }
      return it;
    });
  }

  async function detect() {
    canceling = false;
    detailOpen = false;
    pageState = { kind: "detecting" };
    try {
      const status = await checkEnvironment();
      if (status.allReady) {
        pageState = {
          kind: "ready",
          items: status.items,
          sizeMb: status.installSizeEstimateMb,
        };
        setTimeout(() => navigateTo("queue"), 1000);
        return;
      }

      // Phase 3 guard chain (Design §5.1, Plan FR-04 순차):
      //   missing → check_internet → no-internet
      //           → check_disk_space → disk-full
      //           → installing
      const online = await checkInternet().catch(() => true);
      if (!online) {
        pageState = { kind: "no-internet" };
        return;
      }

      const disk = await checkDiskSpace();
      if (!disk.fits) {
        pageState = {
          kind: "disk-full",
          required: disk.breakdown,
          current: disk.freeMb,
        };
        return;
      }

      // 모든 가드 통과 → installing
      pageState = {
        kind: "installing",
        items: status.items,
        progress: {
          step: "앱을 사용할 준비를 하고 있어요...",
          percent: 0,
          phase: "extract_python",
          currentSizeMb: null,
          estimatedFinalMb: status.installSizeEstimateMb,
        },
      };

      try {
        await installDependencies((p) => {
          if (pageState.kind !== "installing") return;
          pageState = {
            kind: "installing",
            items: applyPhaseToItems(pageState.items, p),
            progress: p,
          };
        });

        // 설치 성공 → 한 번 더 detect (health check) → all_ready 검증
        const reCheck = await checkEnvironment();
        if (reCheck.allReady) {
          pageState = {
            kind: "ready",
            items: reCheck.items,
            sizeMb: reCheck.installSizeEstimateMb,
          };
          setTimeout(() => navigateTo("queue"), 1000);
        } else {
          pageState = {
            kind: "error",
            items: reCheck.items,
            message: "설치는 끝났지만 일부 항목을 확인하지 못했어요. 다시 시도해주세요.",
            detail: "post-install health check failed",
          };
        }
      } catch (e) {
        const raw = String(e);
        pageState = {
          kind: "error",
          items: pageState.kind === "installing" ? pageState.items : status.items,
          message: translateToFriendlyMessage(raw),
          detail: raw,
        };
      }
    } catch (e) {
      const raw = String(e);
      pageState = {
        kind: "error",
        items: [],
        message: translateToFriendlyMessage(raw),
        detail: raw,
      };
    }
  }

  async function handleCancel() {
    if (pageState.kind !== "installing" || canceling) return;
    canceling = true;
    try {
      await cancelInstall();
      // installDependencies가 자체적으로 Err 반환 → 위 catch 블록에서 error 상태 진입
    } catch {
      canceling = false;
    }
  }

  onMount(detect);

  function formatSize(mb: number): string {
    if (mb >= 1024) return `${(mb / 1024).toFixed(1)} GB`;
    return `${mb} MB`;
  }

  async function copyDetail(detail: string) {
    try {
      await navigator.clipboard.writeText(detail);
    } catch {
      // 클립보드 실패 무시
    }
  }
</script>

<div class="flex h-full items-center justify-center p-6">
  <div class="w-full max-w-[600px]">
    {#if pageState.kind === "detecting"}
      <div class="flex flex-col items-center gap-4 py-12" in:fade={{ duration: 200 }}>
        <div class="text-5xl">🎵</div>
        <h1 class="text-2xl font-bold">MR Extractor</h1>
        <p class="text-sm text-muted">환경을 확인하고 있어요...</p>
        <div class="mt-4 h-1 w-48 overflow-hidden rounded-full bg-surface">
          <div class="h-full animate-pulse rounded-full bg-accent" style="width: 60%"></div>
        </div>
      </div>
    {:else if pageState.kind === "ready"}
      <div class="flex flex-col items-center gap-4 py-8" in:fade={{ duration: 200 }}>
        <div class="text-5xl">✨</div>
        <h1 class="text-2xl font-bold">모든 준비가 완료되었어요!</h1>
        <div class="w-full max-w-sm rounded-xl border border-border bg-surface p-4">
          {#each pageState.items as item (item.label)}
            <EnvItemRow {item} />
          {/each}
        </div>
        <p class="text-sm text-muted">📊 사용 중인 공간: {formatSize(pageState.sizeMb)}</p>
        <p class="text-xs text-muted">(추가 모델은 사용 시 자동 다운로드)</p>
      </div>
    {:else if pageState.kind === "installing"}
      <div class="flex flex-col gap-4 py-6" in:fade={{ duration: 200 }}>
        <h1 class="text-center text-xl font-bold">앱을 사용할 준비를 하고 있어요...</h1>
        <div class="rounded-xl border border-border bg-surface p-4">
          {#each pageState.items as item (item.label)}
            <EnvItemRow {item} />
          {/each}
        </div>
        <div class="space-y-1">
          <div class="flex items-center justify-between text-xs text-muted">
            <span>{pageState.progress.step}</span>
            <span>{pageState.progress.percent}%</span>
          </div>
          <div class="h-2 overflow-hidden rounded-full bg-surface">
            <div
              class="h-full rounded-full bg-accent transition-all duration-200"
              style="width: {pageState.progress.percent}%"
            ></div>
          </div>
          {#if pageState.progress.estimatedFinalMb > 0}
            <p class="text-xs text-muted">
              예상: {formatSize(pageState.progress.estimatedFinalMb)}
              {#if pageState.progress.currentSizeMb !== null}
                • 사용 중: {formatSize(pageState.progress.currentSizeMb)}
              {/if}
            </p>
          {/if}
        </div>
        <p class="text-center text-xs text-muted">
          처음 실행 시 한 번만 설치됩니다. 인터넷 필요. (약 3~5분)
        </p>
        <button
          class="mx-auto rounded-lg border border-border bg-surface px-3 py-1.5 text-xs text-muted hover:bg-bg disabled:opacity-50"
          disabled={canceling}
          onclick={handleCancel}
        >
          {canceling ? "취소 중..." : "🛑 취소"}
        </button>
      </div>
    {:else if pageState.kind === "error"}
      <div class="flex flex-col gap-4 py-8" in:fade={{ duration: 200 }}>
        <div class="text-center text-4xl">⚠</div>
        <h1 class="text-center text-xl font-bold">설치 중 문제가 발생했어요</h1>
        <p class="text-center text-sm text-muted">{pageState.message}</p>
        {#if pageState.items.length > 0}
          <div class="rounded-xl border border-border bg-surface p-4">
            {#each pageState.items as item (item.label)}
              <EnvItemRow {item} />
            {/each}
          </div>
        {/if}
        <div class="flex flex-col gap-2">
          <button
            class="rounded-lg border border-border bg-surface px-3 py-2 text-left text-xs text-muted hover:bg-bg"
            onclick={() => (detailOpen = !detailOpen)}
          >
            {detailOpen ? "▲" : "▼"} 오류 상세 보기
          </button>
          {#if detailOpen}
            <pre
              class="max-h-40 overflow-auto whitespace-pre-wrap rounded-lg border border-border bg-bg p-3 font-mono text-xs text-muted">{pageState.detail}</pre>
            <button
              class="self-start text-xs text-accent hover:underline"
              onclick={() => copyDetail(pageState.kind === "error" ? pageState.detail : "")}
            >
              📋 오류 복사
            </button>
          {/if}
        </div>
        <button
          class="rounded-lg bg-accent px-4 py-2 text-sm font-semibold text-white hover:brightness-110"
          onclick={detect}
        >
          🔄 다시 시도
        </button>
      </div>
    {:else if pageState.kind === "no-internet"}
      <div class="flex flex-col items-center gap-4 py-12" in:fade={{ duration: 200 }}>
        <div class="text-5xl">📡</div>
        <h1 class="text-xl font-bold">인터넷 연결이 필요해요</h1>
        <p class="text-center text-sm text-muted">
          처음 사용하시는 경우 앱 구성 요소를 다운로드해야 해요.<br />
          Wi-Fi 또는 유선 연결을 확인해주세요.
        </p>
        <button
          class="rounded-lg bg-accent px-4 py-2 text-sm font-semibold text-white hover:brightness-110"
          onclick={detect}
        >
          🔄 다시 확인
        </button>
      </div>
    {:else if pageState.kind === "disk-full"}
      <div class="flex flex-col gap-4 py-8" in:fade={{ duration: 200 }}>
        <div class="text-center text-4xl">💾</div>
        <h1 class="text-center text-xl font-bold">저장 공간이 부족해요</h1>
        <div class="rounded-xl border border-border bg-surface p-4 text-sm">
          <div class="mb-2 font-semibold">설치 필요</div>
          <div class="flex justify-between py-1">
            <span class="text-muted">음원 분리 엔진</span>
            <span>{formatSize(pageState.required.install)}</span>
          </div>
          <div class="flex justify-between py-1">
            <span class="text-muted">AI 모델</span>
            <span>{formatSize(pageState.required.model)}</span>
          </div>
          <div class="flex justify-between py-1">
            <span class="text-muted">설치 중 임시</span>
            <span>{formatSize(pageState.required.staging)}</span>
          </div>
          <div class="flex justify-between py-1">
            <span class="text-muted">권장 여유</span>
            <span>{formatSize(pageState.required.headroom)}</span>
          </div>
          <hr class="my-2 border-border" />
          <div class="flex justify-between py-1 font-semibold">
            <span>총 필요 공간</span>
            <span>{formatSize(pageState.required.total)}</span>
          </div>
          <div class="flex justify-between py-1 text-danger">
            <span>현재 공간</span>
            <span>{formatSize(pageState.current)} ❌</span>
          </div>
        </div>
        <button
          class="self-center rounded-lg bg-accent px-4 py-2 text-sm font-semibold text-white hover:brightness-110"
          onclick={detect}
        >
          🔄 다시 확인
        </button>
      </div>
    {/if}
  </div>
</div>
