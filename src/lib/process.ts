// Design Ref: ProcessPage Plan §7.2 "Loop 위치: store layer" 확정 구현.
// Plan FR-02 (순차 1개씩) / FR-07 (첫 완료 자동 라우팅 + hasRouted 가드) /
// FR-10 (ProcessPage unmount와 독립 — module-level Promise로 유지) /
// NFR Concurrency (재진입 가드 — 동시 demucs 인스턴스 ≤ 1).
//
// ProcessPage는 이 모듈의 startSeparationBatch를 트리거만 하고, loop 자체는
// 페이지 라이프사이클 밖(module scope)에서 돈다. 진행률은 queueStore(SoT)로만
// 흐르므로 어떤 페이지에서든 subscribe로 관찰 가능 (SC-6/SC-10).

import { get, writable, type Readable } from "svelte/store";
import { queueStore, navigateTo, pushToast } from "./stores";
import { separateAudio } from "./commands";
import { updateQueueItem } from "./queue";

// ─── 배치 상태 (goBack 재진입 시 pagePayload가 null이라 fallback으로 사용) ────
// Plan FR-10: "재진입 시 같은 ids 사용". goBack()은 pagePayload를 비우므로
// 마지막 배치를 store로 보존 — ProcessPage가 payload 부재 시 이걸 읽는다.

export interface ProcessBatch {
  ids: string[];
  model: string;
}

const batchStore = writable<ProcessBatch | null>(null);
export const processBatch: Readable<ProcessBatch | null> = batchStore;

// ─── 재진입/라우팅 가드 (module state — 페이지 unmount와 무관) ────────────────

let running = false;
let hasRouted = false; // FR-07 / SC-17 — 배치당 첫 완료 1회만 자동 라우팅

/** NFR Concurrency — ProcessPage 중복 진입(더블 클릭 등) 방어용. */
export function isSeparationRunning(): boolean {
  return running;
}

// ─── 순차 처리 본체 (Plan FR-02: for-await, 동시 demucs ≤ 1) ─────────────────

/**
 * queue-page payload { ids, model }를 받아 순차 분리.
 * - 항목 실패/취소 → status="error" 후 다음 항목 계속 (SC-8, throw로 loop 중단 금지)
 * - 첫 성공 → PlayerPage 자동 진입 (SC-5), 나머지는 백그라운드 계속 (SC-6)
 */
export async function startSeparationBatch(
  ids: string[],
  model: string,
): Promise<void> {
  if (running) {
    // 재진입 가드 — 진행 중 배치에 합류하지 않음 (동시 다중은 Plan §2.2 Out of Scope).
    // QueuePage isProcessing 가드로 UI상 도달 불가하지만, 무음 드랍 방지용 안내 (code-analyzer fix).
    pushToast("이미 다른 항목을 처리 중이에요. 완료 후 다시 시작해 주세요.", "warn");
    return;
  }
  running = true;
  hasRouted = false;
  batchStore.set({ ids, model });

  try {
    for (const id of ids) {
      const item = get(queueStore).find((it) => it.id === id);
      // 삭제됐거나 이미 처리된 항목은 건너뜀 (배치 재트리거 시 멱등)
      if (!item || item.status !== "ready-to-separate") continue;

      const inputPath = item.tmpPath ?? item.source;
      updateQueueItem(id, {
        status: "in-progress",
        progress: 0,
        step: "모델 로드 중...",
      });

      try {
        const result = await separateAudio(id, inputPath, model, (p) => {
          updateQueueItem(id, { progress: p.percent, step: p.step });
        });
        updateQueueItem(id, {
          status: "done",
          progress: 100,
          step: undefined,
          outputs: {
            vocals: result.vocals,
            drums: result.drums,
            bass: result.bass,
            other: result.other,
          },
        });

        // FR-07 — 배치 첫 완료에만 자동 PlayerPage 진입
        if (!hasRouted) {
          hasRouted = true;
          navigateTo("player", { kind: "player", itemId: id });
        }
      } catch (err) {
        // 취소 포함 — throw가 아닌 error status로 기록하고 다음 항목 계속 (SC-8)
        const msg = typeof err === "string" ? err : String(err);
        updateQueueItem(id, {
          status: "error",
          errorDetail: msg,
          step: undefined,
        });
        pushToast(msg, "error");
      }
    }
  } finally {
    running = false;
  }
}
