use crate::config;

#[tauri::command]
pub async fn health_check(server_url: String) -> Result<bool, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/health", server_url.trim_end_matches('/'));
    match client.get(&url).send().await {
        Ok(response) => Ok(response.status().is_success()),
        Err(_) => Ok(false),
    }
}

#[tauri::command]
pub async fn get_server_url() -> Result<String, String> {
    Ok(config::load_server_url())
}

#[tauri::command]
pub async fn set_server_url(url: String) -> Result<(), String> {
    config::save_server_url(&url).map_err(|e| e.to_string())
}
