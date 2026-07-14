// Design Ref: SETTINGS.md — 설정 저장: Tauri Store (settings.json).
// 알림 토글은 notify.ts가, 기본 내보내기 포맷은 ExportPanel이 소비.

import { Store } from "@tauri-apps/plugin-store";
import { get, writable, type Writable } from "svelte/store";
import type { ExportFormat } from "./types";

export interface AppSettings {
  /// 처리 완료/내보내기 완료 시스템 알림 (SETTINGS.md 알림 설정, 기본 ON)
  notifyEnabled: boolean;
  /// 내보내기 기본 포맷 (SETTINGS.md 내보내기 포맷 설정, 기본 WAV)
  defaultFormat: ExportFormat;
}

const DEFAULTS: AppSettings = {
  notifyEnabled: true,
  defaultFormat: "wav",
};

const STORE_FILE = "settings.json";
const STORE_KEY = "settings";

export const appSettings: Writable<AppSettings> = writable({ ...DEFAULTS });

let storeHandle: Store | null = null;
let hydrated = false;

async function getStore(): Promise<Store> {
  if (!storeHandle) {
    storeHandle = await Store.load(STORE_FILE);
  }
  return storeHandle;
}

/** 앱 시작 시 1회 (App.svelte onMount). 실패해도 기본값으로 동작. */
export async function hydrateSettings(): Promise<void> {
  try {
    const s = await getStore();
    const data = await s.get<Partial<AppSettings>>(STORE_KEY);
    if (data) {
      appSettings.set({ ...DEFAULTS, ...data });
    }
  } catch (err) {
    console.error("settings hydrate failed:", err);
  } finally {
    hydrated = true;
  }
}

/** 설정 변경 + 즉시 영속화 (변경 빈도가 낮아 debounce 불필요). */
export function updateSettings(patch: Partial<AppSettings>): void {
  appSettings.update((cur) => ({ ...cur, ...patch }));
  if (!hydrated) return;
  void (async () => {
    try {
      const s = await getStore();
      await s.set(STORE_KEY, get(appSettings));
      await s.save();
    } catch (err) {
      console.error("settings save failed:", err);
    }
  })();
}
