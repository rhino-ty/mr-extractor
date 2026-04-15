// Design Ref: §4.1 -- 환경 감지 + 자동 설치 (placeholder)

#[tauri::command]
pub async fn check_environment() -> Result<String, String> {
    Ok("not implemented".into())
}

#[tauri::command]
pub async fn install_dependencies() -> Result<String, String> {
    Ok("not implemented".into())
}
