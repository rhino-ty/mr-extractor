// Design Ref: UI.md Web Audio 아키텍처 + CLAUDE.md Audio Architecture.
// AudioContext 싱글턴 (컴포넌트마다 생성 금지 — CLAUDE.md Do Not) — Tone.js 전역
// 컨텍스트 하나로 4 stems 그래프를 구성하고, 페이지 unmount와 무관하게 유지한다.
//
// 그래프: Player(stem)×4 → Gain(stem)×4 → Gain(master) → [PitchShift] → Destination
//   - 키 조절 0이면 PitchShift 바이패스 (품질/지연 손실 방지)
//   - 재생/시크/A-B 루프는 Tone.Transport 기반 (4 트랙 샘플 동기 보장)
// 스템 파일 로드: Tauri asset protocol (convertFileSrc) → fetch → decodeAudioData

import * as Tone from "tone";
import { convertFileSrc } from "@tauri-apps/api/core";
import { get, writable, type Readable, type Writable } from "svelte/store";
import type { StemOutputs } from "./types";

/** PlayerPage(QueueItem) + HistoryPage(HistoryEntry) 양쪽에서 로드 가능한 최소 형태. */
export interface PlayableItem {
  id: string;
  label: string;
  outputs?: StemOutputs;
}

export type StemName = "vocals" | "drums" | "bass" | "other";
export const STEM_ORDER: StemName[] = ["vocals", "drums", "bass", "other"];

export const STEM_META: Record<StemName, { icon: string; label: string }> = {
  vocals: { icon: "🎤", label: "보컬" },
  drums: { icon: "🥁", label: "드럼" },
  bass: { icon: "🎸", label: "베이스" },
  other: { icon: "🎹", label: "그 외" },
};

// ─── UI 상태 stores ──────────────────────────────────────────────────────────

export interface StemSetting {
  volume: number; // 0~100
  muted: boolean;
}

export interface MixerState {
  stems: Record<StemName, StemSetting>;
  master: number; // 0~100
  semitones: number; // -12 ~ +12
}

export interface LoadedTrack {
  status: "idle" | "loading" | "ready" | "error";
  itemId: string | null;
  label: string;
  durationSec: number;
  /** ready 상태에서만 존재 — ExportPanel이 스템 경로로 사용. */
  outputs?: StemOutputs;
  error?: string;
}

function defaultMixer(): MixerState {
  return {
    stems: {
      vocals: { volume: 100, muted: false },
      drums: { volume: 100, muted: false },
      bass: { volume: 100, muted: false },
      other: { volume: 100, muted: false },
    },
    master: 100,
    semitones: 0,
  };
}

const mixerW: Writable<MixerState> = writable(defaultMixer());
export const mixerStore: Readable<MixerState> = mixerW;

const trackW: Writable<LoadedTrack> = writable({
  status: "idle",
  itemId: null,
  label: "",
  durationSec: 0,
});
export const loadedTrack: Readable<LoadedTrack> = trackW;

export const isPlaying: Writable<boolean> = writable(false);

// ─── 엔진 내부 상태 (module singleton) ───────────────────────────────────────

interface Channel {
  player: Tone.Player;
  gain: Tone.Gain;
}

let channels: Partial<Record<StemName, Channel>> = {};
let masterGain: Tone.Gain | null = null;
let pitchShift: Tone.PitchShift | null = null;
let pitchActive = false;
let durationSec = 0;
let peaksCache: number[] | null = null;
let loadToken = 0; // 연속 load 경합 방지 (이전 로드 결과 폐기)

// ─── 로드 / 해제 ─────────────────────────────────────────────────────────────

async function fetchBuffer(path: string): Promise<AudioBuffer> {
  const url = convertFileSrc(path);
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`스템 파일을 읽을 수 없어요 (${res.status})`);
  }
  const raw = await res.arrayBuffer();
  return Tone.getContext().rawContext.decodeAudioData(raw);
}

function disposeGraph(): void {
  Tone.getTransport().stop();
  Tone.getTransport().cancel();
  Tone.getTransport().loop = false;
  for (const ch of Object.values(channels)) {
    ch.player.unsync();
    ch.player.dispose();
    ch.gain.dispose();
  }
  channels = {};
  masterGain?.dispose();
  masterGain = null;
  pitchShift?.dispose();
  pitchShift = null;
  pitchActive = false;
  peaksCache = null;
  isPlaying.set(false);
}

/** 4 stems 로드 + 그래프 구성. 이미 같은 item이 로드돼 있으면 no-op. */
export async function loadItem(item: PlayableItem): Promise<void> {
  const outputs = item.outputs;
  if (!outputs) {
    trackW.set({
      status: "error",
      itemId: item.id,
      label: item.label,
      durationSec: 0,
      error: "분리 결과가 없어요. 먼저 분리를 완료해 주세요.",
    });
    return;
  }
  if (get(trackW).itemId === item.id && get(trackW).status === "ready") return;

  const token = ++loadToken;
  disposeGraph();
  mixerW.set(defaultMixer());
  trackW.set({
    status: "loading",
    itemId: item.id,
    label: item.label,
    durationSec: 0,
  });

  try {
    const buffers = await Promise.all(
      STEM_ORDER.map((name) => fetchBuffer(outputs[name])),
    );
    if (token !== loadToken) return; // 더 최신 로드가 시작됨 — 이 결과 폐기

    durationSec = Math.max(...buffers.map((b) => b.duration));
    masterGain = new Tone.Gain(1);
    masterGain.connect(Tone.getDestination());

    STEM_ORDER.forEach((name, i) => {
      const gain = new Tone.Gain(1);
      const player = new Tone.Player(buffers[i] as unknown as AudioBuffer);
      player.connect(gain);
      gain.connect(masterGain as Tone.Gain);
      player.sync().start(0);
      channels[name] = { player, gain };
    });

    peaksCache = computeMixPeaks(buffers, 2048);
    applyMixer(get(mixerW));

    trackW.set({
      status: "ready",
      itemId: item.id,
      label: item.label,
      durationSec,
      outputs,
    });
  } catch (err) {
    if (token !== loadToken) return;
    disposeGraph();
    trackW.set({
      status: "error",
      itemId: item.id,
      label: item.label,
      durationSec: 0,
      error: err instanceof Error ? err.message : String(err),
    });
  }
}

/** 파형 렌더용 믹스 peaks (0~1 정규화, bucket 단위 최대 진폭). */
function computeMixPeaks(buffers: AudioBuffer[], buckets: number): number[] {
  const maxLen = Math.max(...buffers.map((b) => b.length));
  const chans = buffers.map((b) => b.getChannelData(0));
  const bucketSize = Math.max(1, Math.floor(maxLen / buckets));
  const stride = Math.max(1, Math.floor(bucketSize / 32)); // bucket당 최대 32 샘플만
  const peaks = new Array<number>(buckets).fill(0);
  for (let b = 0; b < buckets; b++) {
    const start = b * bucketSize;
    let peak = 0;
    for (let s = start; s < start + bucketSize && s < maxLen; s += stride) {
      let sum = 0;
      for (const data of chans) {
        if (s < data.length) sum += data[s];
      }
      const abs = Math.abs(sum);
      if (abs > peak) peak = abs;
    }
    peaks[b] = peak;
  }
  const max = Math.max(...peaks, 0.0001);
  return peaks.map((p) => p / max);
}

export function getPeaks(): number[] | null {
  return peaksCache;
}

export function getDuration(): number {
  return durationSec;
}

// ─── 재생 제어 (Transport 기반) ──────────────────────────────────────────────

export async function play(): Promise<void> {
  if (get(trackW).status !== "ready") return;
  await Tone.start(); // 첫 사용자 제스처에서 AudioContext resume
  Tone.getTransport().start();
  isPlaying.set(true);
}

export function pause(): void {
  Tone.getTransport().pause();
  isPlaying.set(false);
}

export async function togglePlay(): Promise<void> {
  if (get(isPlaying)) pause();
  else await play();
}

export function stop(): void {
  Tone.getTransport().stop(); // stop은 position 0으로 리셋
  isPlaying.set(false);
}

export function seek(sec: number): void {
  const clamped = Math.min(Math.max(sec, 0), durationSec);
  Tone.getTransport().seconds = clamped;
}

export function getPosition(): number {
  return Math.min(Tone.getTransport().seconds, durationSec);
}

/** 재생 위치가 곡 끝을 지나면 정지 (rAF 폴링 — WaveformPlayer가 호출). */
export function checkEnded(): boolean {
  if (durationSec > 0 && Tone.getTransport().seconds >= durationSec) {
    stop();
    return true;
  }
  return false;
}

// ─── A-B 구간 반복 ───────────────────────────────────────────────────────────

export function setLoop(a: number, b: number): void {
  const t = Tone.getTransport();
  t.setLoopPoints(Math.max(0, a), Math.min(b, durationSec));
  t.loop = true;
}

export function clearLoop(): void {
  Tone.getTransport().loop = false;
}

// ─── 믹서 (볼륨/뮤트/마스터/피치) ────────────────────────────────────────────

function applyMixer(m: MixerState): void {
  for (const name of STEM_ORDER) {
    const ch = channels[name];
    if (!ch) continue;
    const s = m.stems[name];
    ch.gain.gain.rampTo(s.muted ? 0 : s.volume / 100, 0.05);
  }
  if (masterGain) {
    masterGain.gain.rampTo(m.master / 100, 0.05);
  }
  applyPitchRouting(m.semitones);
}

/** 피치 0 = PitchShift 바이패스, 그 외 = master → PitchShift → destination. */
function applyPitchRouting(semitones: number): void {
  if (!masterGain) return;
  const wantActive = semitones !== 0;

  if (wantActive && !pitchShift) {
    pitchShift = new Tone.PitchShift({ pitch: semitones, windowSize: 0.1 });
    pitchShift.connect(Tone.getDestination());
  }
  if (pitchShift) pitchShift.pitch = semitones;

  if (wantActive !== pitchActive) {
    masterGain.disconnect();
    if (wantActive) {
      masterGain.connect(pitchShift as Tone.PitchShift);
    } else {
      masterGain.connect(Tone.getDestination());
    }
    pitchActive = wantActive;
  }
}

function updateMixer(patch: (m: MixerState) => MixerState): void {
  mixerW.update((m) => {
    const next = patch(m);
    applyMixer(next);
    return next;
  });
}

export function setStemVolume(name: StemName, volume: number): void {
  const v = Math.min(100, Math.max(0, Math.round(volume)));
  updateMixer((m) => ({
    ...m,
    stems: { ...m.stems, [name]: { ...m.stems[name], volume: v } },
  }));
}

export function setStemMuted(name: StemName, muted: boolean): void {
  updateMixer((m) => ({
    ...m,
    stems: { ...m.stems, [name]: { ...m.stems[name], muted } },
  }));
}

/** M 단축키 — 하나라도 켜져 있으면 전체 뮤트, 전부 뮤트면 전체 해제. */
export function toggleMuteAll(): void {
  updateMixer((m) => {
    const anyOn = STEM_ORDER.some((n) => !m.stems[n].muted);
    const stems = { ...m.stems };
    for (const n of STEM_ORDER) stems[n] = { ...stems[n], muted: anyOn };
    return { ...m, stems };
  });
}

export function setMasterVolume(volume: number): void {
  const v = Math.min(100, Math.max(0, Math.round(volume)));
  updateMixer((m) => ({ ...m, master: v }));
}

export function nudgeMasterVolume(delta: number): void {
  setMasterVolume(get(mixerW).master + delta);
}

export function setSemitones(semitones: number): void {
  const s = Math.min(12, Math.max(-12, Math.round(semitones)));
  updateMixer((m) => ({ ...m, semitones: s }));
}
