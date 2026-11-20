//! Opening web links in the user's browser.

use tauri::{AppHandle, Url};

pub fn open(app: &AppHandle, url: &str) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_link(app: AppHandle, url: String) -> Result<(), String> {
    let parsed = Url::parse(&url).map_err(|e| format!("not a link: {e}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(format!("only web links can be opened: {url}"));
    }
    open(&app, parsed.as_str())
}
