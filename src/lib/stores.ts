// Design Ref: §10.3 -- Svelte 5 runes 상태 관리
// 바닐라 Svelte에서 runes는 .svelte 파일 내에서만 사용 가능.
// .ts 파일에서는 $state 사용 불가 → writable store 패턴 사용.

import { writable } from "svelte/store";
import type { PageName } from "./types";

export const page = writable<PageName>("setup");
export const pageHistory = writable<PageName[]>([]);

export function navigateTo(target: PageName) {
  page.update((current) => {
    pageHistory.update((history) => [...history, current]);
    return target;
  });
}

export function goBack() {
  pageHistory.update((history) => {
    const prev = history.pop();
    if (prev) {
      page.set(prev);
    }
    return history;
  });
}
