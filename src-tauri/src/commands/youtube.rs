#[tauri::command]
pub async fn download_youtube(url: String, out_dir: String) -> Result<String, String> {
    Ok(format!("not implemented: {} -> {}", url, out_dir))
}
