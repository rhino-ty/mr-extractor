// Design Ref: §10.3 -- Svelte 5 runes 상태 관리
// 바닐라 Svelte에서 runes는 .svelte 파일 내에서만 사용 가능.
// .ts 파일에서는 $state 사용 불가 → writable store 패턴 사용.

import { writable, derived, type Readable, type Writable } from "svelte/store";
import { Store } from "@tauri-apps/plugin-store";
import { exists } from "@tauri-apps/plugin-fs";
import type {
  ModelId,
  PageName,
  NavigatePayload,
  QueueItem,
  QueueItemStatus,
  Toast,
} from "./types";

// ─── Model Selection (model-selector v1.1) ──────────────────────────────────
// MODEL_SELECTOR.md — 기본값 htdemucs_ft. QueuePage 드롭다운에서 변경.

export const selectedModel: Writable<ModelId> = writable("htdemucs_ft");

// ─── Page Navigation ─────────────────────────────────────────────────────────
// Design Ref: §3.2 — navigateTo(page, payload?) 시그니처 확장
// 기존 호출(setup-page navigateTo("queue"))은 payload optional이라 그대로 동작.

export const page = writable<PageName>("setup");
export const pageHistory = writable<PageName[]>([]);
export const pagePayload = writable<NavigatePayload | null>(null);

export function navigateTo(target: PageName, payload?: NavigatePayload): void {
  page.update((current) => {
    pageHistory.update((history) => [...history, current]);
    pagePayload.set(payload ?? null);
    return target;
  });
}

export function goBack() {
  pageHistory.update((history) => {
    const prev = history.pop();
    if (prev) {
      page.set(prev);
      pagePayload.set(null);
    }
    return history;
  });
}

// ─── Toast Store (Plan §10.2 자체 구현) ──────────────────────────────────────

export const toastStore: Writable<Toast[]> = writable([]);

export function pushToast(
  message: string,
  kind: Toast["kind"] = "info",
  durationMs = 3000,
): void {
  const t: Toast = {
    id: crypto.randomUUID(),
    kind,
    message,
    durationMs,
  };
  toastStore.update((arr) => [...arr, t]);
  setTimeout(() => {
    toastStore.update((arr) => arr.filter((x) => x.id !== t.id));
  }, durationMs);
}

// ─── Queue Store (Plan FR-08, Design §7.4) ───────────────────────────────────

export const queueStore: Writable<QueueItem[]> = writable([]);

// Plan FR-20 / Design §11.3 — frontend isProcessing derived
const PROCESSING_STATUSES: QueueItemStatus[] = [
  "fetching-metadata",
  "downloading",
  "extracting",
  "in-progress",
];

export const isProcessing: Readable<boolean> = derived(queueStore, ($q) =>
  $q.some((it) => PROCESSING_STATUSES.includes(it.status)),
);

// ─── Tauri Store Persistence ─────────────────────────────────────────────────
// Design Ref: §7.4 — pending 항목만 저장, debounce 500ms, version 1 schema.

const STORE_FILE = "queue-store.json";
const STORE_KEY = "queue";
const SCHEMA_VERSION = 1;
const WRITE_DEBOUNCE_MS = 500;

interface PersistedQueue {
  version: number;
  queue: QueueItem[];
}

let storeHandle: Store | null = null;
let writeTimer: ReturnType<typeof setTimeout> | null = null;
let hydrated = false;

async function getStore(): Promise<Store> {
  if (!storeHandle) {
    storeHandle = await Store.load(STORE_FILE);
  }
  return storeHandle;
}

function scheduleWrite(items: QueueItem[]): void {
  if (writeTimer) clearTimeout(writeTimer);
  writeTimer = setTimeout(async () => {
    try {
      const s = await getStore();
      const persisted = items.filter((it) => it.status === "pending");
      const data: PersistedQueue = { version: SCHEMA_VERSION, queue: persisted };
      await s.set(STORE_KEY, data);
      await s.save();
    } catch (err) {
      console.error("queue store write failed:", err);
    }
  }, WRITE_DEBOUNCE_MS);
}

queueStore.subscribe((items) => {
  if (!hydrated) return;
  scheduleWrite(items);
});

// Plan SC-7 / FR-08: 앱 시작 시 hydrate. 파일 mismatch 자동 제거 + toast.
export async function hydrateQueueStore(): Promise<void> {
  try {
    const s = await getStore();
    const data = await s.get<PersistedQueue>(STORE_KEY);

    if (!data) {
      hydrated = true;
      return;
    }

    if (data.version !== SCHEMA_VERSION) {
      pushToast("저장된 큐를 불러올 수 없어요.", "warn");
      await s.set(STORE_KEY, { version: SCHEMA_VERSION, queue: [] });
      await s.save();
      hydrated = true;
      return;
    }

    const validated: QueueItem[] = [];
    let removedCount = 0;
    for (const item of data.queue) {
      if (item.sourceType === "youtube") {
        validated.push(item);
        continue;
      }
      try {
        if (await exists(item.source)) {
          validated.push(item);
        } else {
          removedCount++;
        }
      } catch {
        removedCount++;
      }
    }

    queueStore.set(validated);
    hydrated = true;

    if (removedCount > 0) {
      pushToast(`사라진 파일 ${removedCount}개를 큐에서 제거했어요.`, "info");
    }
  } catch (err) {
    console.error("queue store hydrate failed:", err);
    hydrated = true;
  }
}
