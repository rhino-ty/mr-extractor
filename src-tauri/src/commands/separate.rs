#[tauri::command]
pub async fn separate_audio(file_path: String, model: String, out_dir: String) -> Result<String, String> {
    Ok(format!("not implemented: {} ({}) -> {}", file_path, model, out_dir))
}
