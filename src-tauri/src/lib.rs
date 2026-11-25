mod hq;

use serde::Serialize;
use std::sync::Arc;
use tally_core::library::{Library, PlaylistMeta, Secrets};
use tally_core::proxy::Proxy;
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
    hidden: bool,
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
    hidden: bool,
}

impl From<ChannelRow> for ChannelDto {
    fn from(c: ChannelRow) -> Self {
        Self { id: c.id, tvg_id: c.tvg_id, name: c.name, group: c.group, logo: c.logo, number: c.number, fav: c.fav, hidden: c.hidden }
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
async fn live_groups(lib: Lib<'_>, show_hidden: Option<bool>) -> std::result::Result<Vec<GroupDto>, String> {
    let lib = lib.inner().clone();
    blocking(move || match lib.active_store()? {
        Some(s) => Ok(s.groups_ex(show_hidden.unwrap_or(false))?.into_iter().map(|(name, count, hidden)| GroupDto { name, count, hidden }).collect()),
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

/// Channels in a chosen order (`provider`, `az`, `mine`), paged by offset.
#[tauri::command]
async fn live_channels_sorted(lib: Lib<'_>, group: Option<String>, mode: String, show_hidden: bool, offset: usize, limit: usize) -> std::result::Result<Vec<ChannelDto>, String> {
    let lib = lib.inner().clone();
    blocking(move || {
        let Some(store) = lib.active_store()? else { return Ok(Vec::new()) };
        let rows = store.channels_sorted(group.as_deref().filter(|g| !g.is_empty()), &mode, show_hidden, offset, limit.clamp(1, 6000))?;
        Ok(rows.into_iter().map(Into::into).collect())
    })
    .await
}

#[tauri::command]
async fn reorder_channels(lib: Lib<'_>, ids: Vec<i64>) -> std::result::Result<(), String> {
    let lib = lib.inner().clone();
    blocking(move || match lib.active_store()? {
        Some(mut s) => s.reorder_channels(&ids),
        None => Ok(()),
    })
    .await
}

#[tauri::command]
async fn set_hidden(lib: Lib<'_>, id: i64, hidden: bool) -> std::result::Result<(), String> {
    let lib = lib.inner().clone();
    blocking(move || match lib.active_store()? {
        Some(s) => s.set_hidden(id, hidden),
        None => Ok(()),
    })
    .await
}

#[tauri::command]
async fn reorder_groups(lib: Lib<'_>, names: Vec<String>) -> std::result::Result<(), String> {
    let lib = lib.inner().clone();
    blocking(move || match lib.active_store()? {
        Some(mut s) => s.reorder_groups(&names),
        None => Ok(()),
    })
    .await
}

#[tauri::command]
async fn set_group_hidden(lib: Lib<'_>, name: String, hidden: bool) -> std::result::Result<(), String> {
    let lib = lib.inner().clone();
    blocking(move || match lib.active_store()? {
        Some(s) => s.set_group_hidden(&name, hidden),
        None => Ok(()),
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

#[derive(Serialize)]
struct PlayDto {
    /// A local address served by the app's own proxy. The provider's address and password never leave Rust.
    url: String,
}

#[tauri::command]
async fn play_channel(lib: Lib<'_>, proxy: State<'_, Proxy>, id: i64) -> std::result::Result<PlayDto, String> {
    let (lib, proxy) = (lib.inner().clone(), proxy.inner().clone());
    blocking(move || {
        let store = lib.active_store()?.ok_or_else(|| Error::Account("no playlist".into()))?;
        let ch = store.channel_by_id(id)?.ok_or_else(|| Error::Account("channel not found".into()))?;
        proxy.new_session();
        Ok(PlayDto { url: proxy.live_url_for(&ch.url) })
    })
    .await
}

#[tauri::command]
async fn diagnose_channel(lib: Lib<'_>, proxy: State<'_, Proxy>, id: i64) -> std::result::Result<String, String> {
    let (lib, proxy) = (lib.inner().clone(), proxy.inner().clone());
    blocking(move || {
        let store = lib.active_store()?.ok_or_else(|| Error::Account("no playlist".into()))?;
        let ch = store.channel_by_id(id)?.ok_or_else(|| Error::Account("channel not found".into()))?;
        let mut report = tally_core::proxy::diagnose(&ch.url);
        let last = proxy.last_problem();
        if !last.is_empty() {
            report.push_str(&format!("\nlast problem seen while playing: {last}"));
        }
        Ok(report)
    })
    .await
}

// ---- movies and series

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TitleDto {
    kind: String,
    id: String,
    name: String,
    group: String,
    poster: String,
    plot: String,
    rating: f64,
    year: String,
    ext: String,
}

fn title_dto(t: tally_core::model::Title, proxy: &Proxy) -> TitleDto {
    TitleDto {
        kind: t.kind,
        id: t.ext_id,
        name: t.name,
        group: t.group,
        poster: if t.poster.starts_with("http") { proxy.url_for(&t.poster) } else { String::new() },
        plot: t.plot,
        rating: t.rating,
        year: t.year,
        ext: t.ext,
    }
}

#[derive(Serialize)]
struct RowDto {
    /// Empty for the "recently added" row; the UI gives it a localised name.
    name: String,
    recent: bool,
    count: i64,
    items: Vec<TitleDto>,
}

/// Shelves for the Movies / Series screens: recently added first, then each category.
/// Downloads the catalogue first if this playlist never had it.
#[tauri::command]
async fn title_rows(lib: Lib<'_>, proxy: State<'_, Proxy>, kind: String, per_row: usize, max_rows: usize) -> std::result::Result<Vec<RowDto>, String> {
    let (lib, proxy) = (lib.inner().clone(), proxy.inner().clone());
    blocking(move || {
        let Some(mut store) = lib.active_store()? else { return Ok(Vec::new()) };
        if store.title_count(&kind)? == 0 {
            lib.sync_titles()?;
            store = lib.active_store()?.ok_or_else(|| Error::Account("no playlist".into()))?;
        }
        let mut rows = Vec::new();
        let recent = store.titles_recent(&kind, per_row)?;
        if !recent.is_empty() {
            rows.push(RowDto { name: String::new(), recent: true, count: store.title_count(&kind)?, items: recent.into_iter().map(|t| title_dto(t, &proxy)).collect() });
        }
        for (g, count) in store.title_groups(&kind)?.into_iter().take(max_rows) {
            let items = store.titles_in(&kind, Some(&g), 0, per_row)?;
            rows.push(RowDto { name: g, recent: false, count, items: items.into_iter().map(|t| title_dto(t, &proxy)).collect() });
        }
        Ok(rows)
    })
    .await
}

/// A category as a grid (paged), or search results when `query` is set.
#[tauri::command]
async fn title_page(lib: Lib<'_>, proxy: State<'_, Proxy>, kind: String, group: Option<String>, query: Option<String>, offset: usize, limit: usize) -> std::result::Result<Vec<TitleDto>, String> {
    let (lib, proxy) = (lib.inner().clone(), proxy.inner().clone());
    blocking(move || {
        let Some(store) = lib.active_store()? else { return Ok(Vec::new()) };
        let list = match query.as_deref().map(str::trim).filter(|q| !q.is_empty()) {
            Some(q) => store.titles_search(&kind, q, limit)?,
            None if group.is_none() && offset == 0 && limit == 0 => Vec::new(),
            None => store.titles_in(&kind, group.as_deref(), offset, limit)?,
        };
        Ok(list.into_iter().map(|t| title_dto(t, &proxy)).collect())
    })
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EpisodeDto {
    id: String,
    season: u32,
    number: u32,
    title: String,
    plot: String,
    duration: String,
    ext: String,
}

#[derive(Serialize)]
struct SeriesDto {
    name: String,
    plot: String,
    episodes: Vec<EpisodeDto>,
}

#[tauri::command]
async fn series_detail(lib: Lib<'_>, id: String) -> std::result::Result<SeriesDto, String> {
    let lib = lib.inner().clone();
    blocking(move || {
        let client = lib.client()?.ok_or_else(|| Error::Account("no playlist".into()))?;
        let d = client.series_detail(&id)?;
        Ok(SeriesDto {
            name: d.name,
            plot: d.plot,
            episodes: d.episodes.into_iter().map(|e| EpisodeDto { id: e.id, season: e.season, number: e.number, title: e.title, plot: e.plot, duration: e.duration, ext: e.ext }).collect(),
        })
    })
    .await
}

/// `kind` is `movie` or `episode`. The address stays here; the UI only receives the local one.
#[tauri::command]
async fn play_title(lib: Lib<'_>, proxy: State<'_, Proxy>, kind: String, id: String, ext: String) -> std::result::Result<PlayDto, String> {
    let (lib, proxy) = (lib.inner().clone(), proxy.inner().clone());
    blocking(move || {
        let client = lib.client()?.ok_or_else(|| Error::Account("no playlist".into()))?;
        let url = if kind == "movie" { client.movie_url(&id, &ext) } else { client.episode_url(&id, &ext) };
        proxy.new_session();
        Ok(PlayDto { url: proxy.url_for(&url) })
    })
    .await
}

// ---- high-quality player (libmpv)

#[tauri::command]
fn hq_available(hq: State<'_, Arc<hq::Hq>>) -> bool {
    hq.available()
}

#[tauri::command]
fn hq_play(
    app: tauri::AppHandle,
    hq: State<'_, Arc<hq::Hq>>,
    lib: Lib<'_>,
    proxy: State<'_, Proxy>,
    items: Vec<hq::Item>,
    index: usize,
    options: hq::Options,
) -> std::result::Result<(), String> {
    hq.inner().play(&app, lib.inner(), proxy.inner(), items, index, options).map_err(|e| e.to_string())
}

#[tauri::command]
fn hq_stop(hq: State<'_, Arc<hq::Hq>>) {
    hq.stop();
}

#[tauri::command]
async fn live_favorites(lib: Lib<'_>) -> std::result::Result<Vec<ChannelDto>, String> {
    let lib = lib.inner().clone();
    blocking(move || match lib.active_store()? {
        Some(s) => Ok(s.favorites()?.into_iter().map(Into::into).collect()),
        None => Ok(Vec::new()),
    })
    .await
}

#[tauri::command]
async fn reorder_favorites(lib: Lib<'_>, ids: Vec<i64>) -> std::result::Result<(), String> {
    let lib = lib.inner().clone();
    blocking(move || match lib.active_store()? {
        Some(mut s) => s.reorder_favorites(&ids),
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
            app.manage(Proxy::start().map_err(|e| e.to_string())?);
            // the player library may sit next to the app or in its resources
            let mut dirs = Vec::new();
            if let Ok(exe) = std::env::current_exe() {
                if let Some(d) = exe.parent() {
                    dirs.push(d.to_path_buf());
                    dirs.push(d.join("resources"));
                }
            }
            if let Ok(r) = app.path().resource_dir() {
                dirs.push(r.clone());
                dirs.push(r.join("resources"));
            }
            app.manage(Arc::new(hq::Hq::new(dirs)));
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
            live_channels_sorted,
            reorder_channels,
            set_hidden,
            reorder_groups,
            set_group_hidden,
            set_favorite,
            play_channel,
            diagnose_channel,
            hq_available,
            hq_play,
            hq_stop,
            title_rows,
            title_page,
            series_detail,
            play_title,
            live_favorites,
            reorder_favorites,
            core_version
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tally");
}
