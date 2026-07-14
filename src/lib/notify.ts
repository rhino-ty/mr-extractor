// ROADMAP v1 필수 — 처리 완료 시 시스템 알림. 권한 체크 공통화 (ExportPanel/process.ts 공유).

import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";

/** 시스템 알림 (best-effort — 실패해도 앱 흐름에 영향 없음). */
export async function notify(title: string, body: string): Promise<void> {
  try {
    let granted = await isPermissionGranted();
    if (!granted) {
      granted = (await requestPermission()) === "granted";
    }
    if (granted) {
      sendNotification({ title, body });
    }
  } catch {
    // 알림 실패는 조용히 무시 (토스트가 주 채널)
  }
}
