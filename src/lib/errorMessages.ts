// Design Ref: §6.2 — Error Response Format. Plan SC-8: 사용자 노출은 한국어 친절 메시지만,
// 원본은 [▼ 상세] 토글에서만 보이도록.
//
// Rust setup::translate_error 에서 1차 매핑이 일어나지만, IPC 외 채널 (Channel 페이로드,
// 라이브러리 raw 에러 등)이 들어올 수 있어 frontend 2차 방어로 동일 패턴을 다시 적용한다.

const PATTERNS: Array<[RegExp, string]> = [
  [/no space left|disk.?full|저장 공간이 부족/i, "저장 공간이 부족해요. 정리 후 다시 시도해주세요."],
  [
    /connection ?(error|reset|aborted)|timeout|tls|dns|연결이 끊|connect/i,
    "인터넷 연결이 끊겼어요. 다시 시도해주세요.",
  ],
  [
    /access ?denied|permission|권한이 없|os error 5/i,
    "파일 쓰기 권한이 없어요. 관리자 권한으로 실행하거나 백신 예외에 추가해주세요.",
  ],
  [
    /antivirus|defender|백신|smartscreen/i,
    "백신 프로그램이 앱 파일을 차단하고 있어요. 예외 처리 후 다시 시도해주세요.",
  ],
  [
    /cancel|취소|aborted by user/i,
    "설치가 취소되었어요. 다시 시도하려면 [🔄 다시 시도]를 눌러주세요.",
  ],
];

const FALLBACK = "설치 중 문제가 발생했어요. 다시 시도해주세요.";

/**
 * raw error string → 사용자 친화 한국어 메시지.
 * 매칭 실패 시 fallback. 원본은 호출자가 detail 필드에 보존해야 한다.
 */
export function translateToFriendlyMessage(raw: string): string {
  if (!raw) return FALLBACK;
  // Rust translate_error가 이미 친절 메시지로 매핑한 경우(한국어 키워드 포함)는 그대로 통과.
  if (raw.includes("부족해요") || raw.includes("끊겼어요") || raw.includes("권한이 없어요") || raw.includes("차단하고 있어요") || raw.includes("취소되었어요")) {
    return raw;
  }
  for (const [pattern, message] of PATTERNS) {
    if (pattern.test(raw)) return message;
  }
  return FALLBACK;
}
