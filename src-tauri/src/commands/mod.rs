// Design Ref: §2.3 Dependencies. common은 다른 commands 없이 standalone — foundation layer.
pub mod common;
pub mod setup;
pub mod youtube;
pub mod separate;
pub mod video;
pub mod export;
pub mod queue;  // queue-page Phase 2 — QueueHandle State (Phase 3 cancel_queue_item 추가 예정)
pub mod model;  // model-selector v1.1 — 모델 목록 + on-demand 다운로드
pub mod settings;  // settings-page v1.2 — 저장 공간 현황 + 임시 파일 정리
