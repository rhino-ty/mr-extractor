// Design Ref: §3.1 -- 이 피처에서는 라우팅 + 환경 상태 타입만 정의

export type PageName =
  | "setup"
  | "queue"
  | "process"
  | "player"
  | "history"
  | "settings";

export type EnvItemStatus = "ready" | "missing" | "installing" | "error";

export interface EnvItem {
  label: string;
  status: EnvItemStatus;
  version?: string;
}
