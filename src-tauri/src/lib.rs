use serde::Serialize;
use tally_core::{Store, xtream::XtreamClient};
use tauri::Manager;

/// What the UI is told after a successful connection. Never contains credentials or stream URLs.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Connected {
    status: String,
    expires_at: Option<i64>,
    trial: bool,
    max_connections: Option<u32>,
    active_connections: Option<u32>,
    channels: usize,
}

/// Log in to an Xtream Codes provider, download the channel list and index it.
/// Credentials live only in this function's memory; persistent (keychain) storage comes next.
/// Errors come from `tally-core`, which already removes the username and password from messages.
#[tauri::command]
async fn connect_xtream(app: tauri::AppHandle, host: String, user: String, pass: String) -> Result<Connected, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || -> Result<Connected, String> {
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let client = XtreamClient::new(&host, &user, &pass);
        let acct = client.account().map_err(|e| e.to_string())?;
        let mut list = Vec::new();
        client.playlist(|c| list.push(c)).map_err(|e| e.to_string())?;
        let mut store = Store::open(&dir.join("tally.db")).map_err(|e| e.to_string())?;
        let channels = store.import_channels(list).map_err(|e| e.to_string())?;
        Ok(Connected {
            status: acct.status,
            expires_at: acct.expires_at,
            trial: acct.is_trial,
            max_connections: acct.max_connections,
            active_connections: acct.active_connections,
            channels,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn core_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![connect_xtream, core_version])
        .run(tauri::generate_context!())
        .expect("error while running Tally");
}
