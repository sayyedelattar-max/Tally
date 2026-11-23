use serde::Serialize;
use std::sync::Arc;
use tally_core::library::{Library, PlaylistMeta, Secrets};
use tally_core::store::ChannelRow;
use tally_core::{Error, Result};
use tauri::{Manager, State};

type Lib<'a> = State<'a, Arc<Library>>;

/// Passwords live in the Windows Credential Manager, one entry per playlist.
struct Keychain;

impl Keychain {
    fn entry(id: &str) -> std::result::Result<keyring::Entry, keyring::Error> {
        keyring::Entry::new("app.tally.player", id)
    }
}

impl Secrets for Keychain {
    fn get(&self, id: &str) -> Option<String> {
        Self::entry(id).ok()?.get_password().ok()
    }
    fn set(&self, id: &str, secret: &str) -> Result<()> {
        Self::entry(id)
            .and_then(|e| e.set_password(secret))
            .map_err(|e| Error::Account(format!("could not store the password in the system credential store: {e}")))
    }
    fn delete(&self, id: &str) {
        if let Ok(e) = Self::entry(id) {
            let _ = e.delete_credential();
        }
    }
}

// What the UI receives. Never contains passwords or stream addresses.

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaylistDto {
    id: String,
    name: String,
    host: String,
    user: String,
    status: String,
    expires_at: Option<i64>,
    trial: bool,
    max_connections: Option<u32>,
    active_connections: Option<u32>,
    channels: i64,
    updated_at: i64,
}

impl From<PlaylistMeta> for PlaylistDto {
    fn from(m: PlaylistMeta) -> Self {
        Self {
            id: m.id,
            name: m.name,
            host: m.host,
            user: m.user,
            status: m.status,
            expires_at: m.expires_at,
            trial: m.trial,
            max_connections: m.max_connections,
            active_connections: m.active_connections,
            channels: m.channels,
            updated_at: m.updated_at,
        }
    }
}

#[derive(Serialize)]
struct RegistryDto {
    active: Option<String>,
    items: Vec<PlaylistDto>,
}

#[derive(Serialize)]
struct GroupDto {
    name: String,
    count: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChannelDto {
    id: i64,
    tvg_id: String,
    name: String,
    group: String,
    logo: String,
    number: Option<i64>,
    fav: bool,
}

impl From<ChannelRow> for ChannelDto {
    fn from(c: ChannelRow) -> Self {
        Self { id: c.id, tvg_id: c.tvg_id, name: c.name, group: c.group, logo: c.logo, number: c.number, fav: c.fav }
    }
}

#[derive(Serialize)]
struct ChannelPage {
    rows: Vec<ChannelDto>,
    /// Pass back as `after` to get the next page; `None` when there is no more.
    next: Option<(i64, i64)>,
}

/// Run blocking work (network, SQLite) off the UI thread and turn errors into text.
async fn blocking<T, F>(f: F) -> std::result::Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || f().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn playlists_list(lib: Lib<'_>) -> std::result::Result<RegistryDto, String> {
    let lib = lib.inner().clone();
    blocking(move || {
        let (active, items) = lib.list()?;
        Ok(RegistryDto { active, items: items.into_iter().map(Into::into).collect() })
    })
    .await
}

/// Add (`id` empty) or edit a playlist. Contacts the server first; nothing is saved on failure.
#[tauri::command]
async fn playlist_save(
    lib: Lib<'_>,
    id: Option<String>,
    name: String,
    host: String,
    user: String,
    pass: Option<String>,
) -> std::result::Result<PlaylistDto, String> {
    let lib = lib.inner().clone();
    blocking(move || Ok(lib.save(id.as_deref(), &name, &host, &user, pass.as_deref())?.into())).await
}

#[tauri::command]
async fn playlist_refresh(lib: Lib<'_>, id: String) -> std::result::Result<PlaylistDto, String> {
    let lib = lib.inner().clone();
    blocking(move || Ok(lib.refresh(&id)?.into())).await
}

#[tauri::command]
async fn playlist_remove(lib: Lib<'_>, id: String) -> std::result::Result<(), String> {
    let lib = lib.inner().clone();
    blocking(move || lib.remove(&id)).await
}

#[tauri::command]
async fn playlist_activate(lib: Lib<'_>, id: String) -> std::result::Result<(), String> {
    let lib = lib.inner().clone();
    blocking(move || lib.activate(&id)).await
}

#[tauri::command]
async fn live_groups(lib: Lib<'_>) -> std::result::Result<Vec<GroupDto>, String> {
    let lib = lib.inner().clone();
    blocking(move || match lib.active_store()? {
        Some(s) => Ok(s.groups()?.into_iter().map(|(name, count)| GroupDto { name, count }).collect()),
        None => Ok(Vec::new()),
    })
    .await
}

/// One page of channels: a search when `query` is given, otherwise the list in channel-number order,
/// optionally limited to one group.
#[tauri::command]
async fn live_channels(
    lib: Lib<'_>,
    group: Option<String>,
    query: Option<String>,
    after: Option<(i64, i64)>,
    limit: usize,
) -> std::result::Result<ChannelPage, String> {
    let lib = lib.inner().clone();
    blocking(move || {
        let Some(store) = lib.active_store()? else {
            return Ok(ChannelPage { rows: Vec::new(), next: None });
        };
        let limit = limit.clamp(1, 500);
        let q = query.unwrap_or_default();
        if !q.trim().is_empty() {
            let rows = store.search(&q, limit)?.channels;
            return Ok(ChannelPage { rows: rows.into_iter().map(Into::into).collect(), next: None });
        }
        let rows = match group.as_deref().filter(|g| !g.is_empty()) {
            Some(g) => store.channels_in_group_after(g, after, limit)?,
            None => store.channels_after(after, limit)?,
        };
        let next = if rows.len() == limit { rows.last().and_then(|c| c.number.map(|n| (n, c.id))) } else { None };
        Ok(ChannelPage { rows: rows.into_iter().map(Into::into).collect(), next })
    })
    .await
}

#[tauri::command]
async fn set_favorite(lib: Lib<'_>, id: i64, fav: bool) -> std::result::Result<(), String> {
    let lib = lib.inner().clone();
    blocking(move || match lib.active_store()? {
        Some(s) => s.set_favorite(id, fav),
        None => Ok(()),
    })
    .await
}

#[tauri::command]
fn core_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            app.manage(Arc::new(Library::new(dir, Box::new(Keychain))));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            playlists_list,
            playlist_save,
            playlist_refresh,
            playlist_remove,
            playlist_activate,
            live_groups,
            live_channels,
            set_favorite,
            core_version
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tally");
}
