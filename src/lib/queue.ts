// Design Ref: §11.3 Phase 1 Interface Contract — queue 헬퍼 모음
// Plan FR-14 / FR-11 — URL 정규화 + 파일 분류 + 중복 체크 + 큐 add/remove

import { get } from "svelte/store";
import { open } from "@tauri-apps/plugin-dialog";
import { queueStore, pushToast } from "./stores";
import { cancelQueueItem, fetchVideoMetadata, fetchYoutubeMetadata } from "./commands";
import type { QueueItem, QueueItemStatus, QueueSourceType } from "./types";

const PROCESSING_STATUSES_FOR_CANCEL = new Set<QueueItemStatus>([
  "fetching-metadata",
  "downloading",
  "extracting",
]);

// ─── 확장자 (ref FILE_FORMATS.md) ────────────────────────────────────────────

const AUDIO_EXTS = new Set([
  ".mp3", ".wav", ".flac", ".ogg", ".m4a", ".aac",
  ".opus", ".aiff", ".wma", ".ape",
]);
const VIDEO_EXTS = new Set([
  ".mp4", ".mkv", ".mov", ".avi", ".webm",
  ".wmv", ".flv", ".ts", ".m2ts",
]);

// Plan FR-14 — youtube.com / youtu.be / m.youtube.com / music.youtube.com 통일
const YOUTUBE_HOSTS = new Set([
  "youtube.com",
  "www.youtube.com",
  "m.youtube.com",
  "music.youtube.com",
  "youtu.be",
]);

const VIDEO_ID_PATTERN = /^[A-Za-z0-9_-]{11}$/;

// ─── URL 정규화 ──────────────────────────────────────────────────────────────

export function normalizeUrl(raw: string): string | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;

  let withProtocol = trimmed;
  if (!/^https?:\/\//i.test(trimmed)) {
    withProtocol = `https://${trimmed}`;
  }

  let url: URL;
  try {
    url = new URL(withProtocol);
  } catch {
    return null;
  }

  const host = url.hostname.toLowerCase();
  if (!YOUTUBE_HOSTS.has(host)) return null;

  let videoId: string | null = null;

  if (host === "youtu.be") {
    videoId = url.pathname.slice(1).split("/")[0] || null;
  } else if (url.pathname === "/watch") {
    videoId = url.searchParams.get("v");
  } else if (url.pathname.startsWith("/embed/")) {
    videoId = url.pathname.slice("/embed/".length).split("/")[0] || null;
  } else if (url.pathname.startsWith("/v/")) {
    videoId = url.pathname.slice("/v/".length).split("/")[0] || null;
  } else if (url.pathname.startsWith("/shorts/")) {
    videoId = url.pathname.slice("/shorts/".length).split("/")[0] || null;
  }

  if (!videoId || !VIDEO_ID_PATTERN.test(videoId)) return null;

  return `https://youtube.com/watch?v=${videoId}`;
}

// ─── 파일 분류 ───────────────────────────────────────────────────────────────

function getExt(path: string): string {
  const idx = path.lastIndexOf(".");
  if (idx < 0) return "";
  return path.slice(idx).toLowerCase();
}

export function classifyFile(path: string): "audio" | "video" | "unknown" {
  const ext = getExt(path);
  if (AUDIO_EXTS.has(ext)) return "audio";
  if (VIDEO_EXTS.has(ext)) return "video";
  return "unknown";
}

export function getBasename(path: string): string {
  const sepIdx = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return sepIdx >= 0 ? path.slice(sepIdx + 1) : path;
}

export function formatDuration(sec: number | undefined): string {
  if (!sec || sec <= 0) return "";
  const m = Math.floor(sec / 60);
  const s = Math.floor(sec % 60).toString().padStart(2, "0");
  return `${m}:${s}`;
}

// ─── 중복 체크 (Plan FR-11) ──────────────────────────────────────────────────

export function isDuplicate(candidate: QueueItem, queue: QueueItem[]): boolean {
  return queue.some(
    (it) =>
      it.sourceType === candidate.sourceType && it.source === candidate.source,
  );
}

// ─── 큐 추가/제거 (Plan FR-11 / FR-04 / Design §11.3) ────────────────────────

interface AddOptions {
  sourceType: QueueSourceType;
  source: string;
  label: string;
  durationSec?: number;
}

// Plan SC-1/SC-2: 1초 내 placeholder 카드 등장. file/video는 fetching-metadata로 시작 후 자동 fetch.
export function addToQueue(opts: AddOptions): QueueItem | null {
  // Plan §7.3 — file/video는 즉시 ffprobe 호출, youtube는 Phase 3에서 yt-dlp metadata
  const initialStatus: QueueItemStatus =
    opts.sourceType === "youtube" ? "pending" : "fetching-metadata";

  const item: QueueItem = {
    id: crypto.randomUUID(),
    sourceType: opts.sourceType,
    source: opts.source,
    label: opts.label,
    durationSec: opts.durationSec,
    progress: 0,
    status: initialStatus,
    addedAt: new Date().toISOString(),
  };

  const current = get(queueStore);
  if (isDuplicate(item, current)) {
    pushToast("이미 큐에 있어요", "info");
    return null;
  }

  queueStore.update((arr) => [...arr, item]);

  // Phase 2 — file/video는 백그라운드 metadata fetch (시나리오 B)
  if (opts.sourceType === "file" || opts.sourceType === "video") {
    void runVideoMetadataFetch(item.id, opts.source);
  }
  // Phase 3 — youtube는 백그라운드 yt-dlp metadata fetch (시나리오 A)
  // initialStatus가 이미 "pending"이므로 fetching-metadata로 변경 후 fetch
  if (opts.sourceType === "youtube") {
    updateQueueItem(item.id, { status: "fetching-metadata" });
    void runYoutubeMetadataFetch(item.id, opts.source);
  }

  return item;
}

async function runVideoMetadataFetch(id: string, path: string): Promise<void> {
  try {
    const meta = await fetchVideoMetadata(id, path);
    const base = getBasename(path);
    const dur = formatDuration(meta.durationSec);
    updateQueueItem(id, {
      durationSec: meta.durationSec,
      label: dur ? `${base} (${dur})` : base,
      status: "pending",
    });
  } catch (err) {
    const msg = typeof err === "string" ? err : String(err);
    updateQueueItem(id, {
      status: "error",
      errorDetail: msg,
    });
    pushToast(msg.includes("읽을 수 없") ? msg : `정보 가져오기 실패: ${msg}`, "error");
  }
}

// Plan FR-17 / SC-19 — yt-dlp --skip-download 사전 추출 (~3~5초). 5초 초과 시 fallback 라벨 유지.
async function runYoutubeMetadataFetch(id: string, url: string): Promise<void> {
  try {
    const meta = await fetchYoutubeMetadata(id, url);
    const dur = formatDuration(meta.durationSec);
    const title = meta.title.trim() || url;
    updateQueueItem(id, {
      durationSec: meta.durationSec,
      label: dur ? `${title} (${dur})` : title,
      status: "pending",
    });
  } catch (err) {
    const msg = typeof err === "string" ? err : String(err);
    updateQueueItem(id, {
      status: "error",
      errorDetail: msg,
    });
    pushToast(msg, "error");
  }
}

export function updateQueueItem(id: string, patch: Partial<QueueItem>): void {
  queueStore.update((arr) =>
    arr.map((it) => (it.id === id ? { ...it, ...patch } : it)),
  );
}

// Plan FR-04 / FR-18 — Phase 3: 처리 중이면 cancel_queue_item 호출 후 제거 (fix IV).
// 일반 pending/error/done 항목은 단순 제거.
export async function removeFromQueue(id: string): Promise<void> {
  const current = get(queueStore);
  const item = current.find((it) => it.id === id);
  if (item && PROCESSING_STATUSES_FOR_CANCEL.has(item.status)) {
    try {
      await cancelQueueItem(id);
    } catch (err) {
      console.warn("cancel_queue_item failed (non-fatal):", err);
    }
  }
  queueStore.update((arr) => arr.filter((it) => it.id !== id));
}

export async function removeManyFromQueue(ids: string[]): Promise<void> {
  const idSet = new Set(ids);
  const current = get(queueStore);
  // 처리 중 항목은 cancel 먼저 (병렬). 단순 제거는 한 번에.
  const cancelTargets = current.filter(
    (it) => idSet.has(it.id) && PROCESSING_STATUSES_FOR_CANCEL.has(it.status),
  );
  await Promise.allSettled(cancelTargets.map((it) => cancelQueueItem(it.id)));
  queueStore.update((arr) => arr.filter((it) => !idSet.has(it.id)));
}

// ─── 드래그/드롭 + 파일 다이얼로그 공통 처리 ─────────────────────────────────
// Plan FR-02 — 다중 파일 동시 처리 (DropZone + Ctrl+O 공유)

export function processDroppedPaths(paths: string[]): void {
  let unsupported = 0;
  for (const path of paths) {
    const kind = classifyFile(path);
    if (kind === "unknown") {
      unsupported++;
      continue;
    }
    addToQueue({
      sourceType: kind === "audio" ? "file" : "video",
      source: path,
      label: getBasename(path),
    });
  }
  if (unsupported > 0) {
    pushToast(
      unsupported === 1
        ? "지원하지 않는 파일 형식이에요"
        : `지원하지 않는 파일 ${unsupported}개를 건너뛰었어요`,
      "warn",
    );
  }
}

export async function openFileDialog(): Promise<void> {
  try {
    const selected = await open({
      multiple: true,
      directory: false,
      filters: [
        {
          name: "오디오/영상 파일",
          extensions: [
            "mp3", "wav", "flac", "ogg", "m4a", "aac", "opus", "aiff", "wma", "ape",
            "mp4", "mkv", "mov", "avi", "webm", "wmv", "flv", "ts", "m2ts",
          ],
        },
      ],
    });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    processDroppedPaths(paths);
  } catch (err) {
    console.error("file dialog failed:", err);
    pushToast("파일 다이얼로그를 열 수 없어요", "error");
  }
}
