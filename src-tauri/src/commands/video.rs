#[tauri::command]
pub async fn extract_audio(video_path: String, out_dir: String) -> Result<String, String> {
    Ok(format!("not implemented: {} -> {}", video_path, out_dir))
}
