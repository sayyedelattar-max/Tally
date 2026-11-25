//! The high-quality player: libmpv in its own window, driven from here.
//!
//! Channel and episode changes come back as key presses (`script-message`), so zapping works the
//! same way as in the built-in player, and the address of a stream never reaches the web page.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Deserialize;
use tally_core::library::Library;
use tally_core::mpv::{Api0, Event, Mpv};
use tally_core::proxy::Proxy;
use tally_core::{Error, Result};
use tauri::{AppHandle, Emitter};

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub name: String,
    pub live_id: Option<i64>,
    pub vod: Option<VodRef>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct VodRef {
    pub kind: String,
    pub id: String,
    pub ext: String,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Options {
    /// `standard`, `high` or `ultra`.
    pub quality: String,
    /// Seconds of video to keep ahead.
    pub buffer_secs: u32,
    pub fullscreen: bool,
}

struct Session {
    mpv: Arc<Mpv>,
    state: Mutex<State>,
}

struct State {
    items: Vec<Item>,
    index: usize,
    retries: u32,
    fit: u8,
}

pub struct Hq {
    dirs: Vec<PathBuf>,
    api: Mutex<Option<Option<Api0>>>,
    session: Mutex<Option<Arc<Session>>>,
}

impl Hq {
    pub fn new(dirs: Vec<PathBuf>) -> Self {
        Hq { dirs, api: Mutex::new(None), session: Mutex::new(None) }
    }

    fn api(&self) -> Result<Api0> {
        let mut g = self.api.lock().unwrap();
        if g.is_none() {
            *g = Some(Mpv::load(&self.dirs).ok());
        }
        g.as_ref().and_then(|a| a.clone()).ok_or_else(|| Error::Account("the high-quality player (libmpv) is not installed".into()))
    }

    pub fn available(&self) -> bool {
        self.api().is_ok()
    }

    pub fn stop(&self) {
        let s = self.session.lock().unwrap().clone();
        if let Some(s) = s {
            let _ = s.mpv.command(&["quit"]);
        }
    }

    /// Start (or continue) playing `items` from `index`.
    pub fn play(self: &Arc<Self>, app: &AppHandle, lib: &Arc<Library>, proxy: &Proxy, items: Vec<Item>, index: usize, opts: Options) -> Result<()> {
        if items.is_empty() {
            return Err(Error::Account("nothing to play".into()));
        }
        let index = index.min(items.len() - 1);
        let existing = self.session.lock().unwrap().clone();
        if let Some(s) = existing {
            // a window is already open: reuse it
            {
                let mut st = s.state.lock().unwrap();
                st.items = items;
                st.index = index;
                st.retries = 0;
            }
            return load_current(&s, lib, proxy);
        }
        let api = self.api()?;
        let mpv = Arc::new(api.create()?);
        configure(&mpv, &opts)?;
        mpv.init()?;
        for (key, msg) in [
            ("UP", "tally-up"), ("DOWN", "tally-down"), ("PGUP", "tally-prev"), ("PGDWN", "tally-next"),
            ("<", "tally-prev"), (">", "tally-next"), ("MBTN_BACK", "tally-prev"), ("MBTN_FORWARD", "tally-next"),
            ("z", "tally-fit"), ("ESC", "tally-esc"),
        ] {
            let _ = mpv.command(&["keybind", key, &format!("script-message {msg}")]);
        }
        let session = Arc::new(Session { mpv: mpv.clone(), state: Mutex::new(State { items, index, retries: 0, fit: 0 }) });
        *self.session.lock().unwrap() = Some(session.clone());
        let (me, app, lib, proxy) = (self.clone(), app.clone(), lib.clone(), proxy.clone());
        std::thread::spawn(move || {
            let _ = load_current(&session, &lib, &proxy);
            let _ = app.emit("hq-index", index);
            loop {
                match session.mpv.wait(0.5) {
                    Event::Shutdown => break,
                    Event::FileLoaded => session.state.lock().unwrap().retries = 0,
                    Event::Message(m) => {
                        let Some(name) = m.first() else { continue };
                        let live = {
                            let st = session.state.lock().unwrap();
                            st.items[st.index].live_id.is_some()
                        };
                        match (name.as_str(), live) {
                            ("tally-prev", _) | ("tally-up", true) => step(&session, &lib, &proxy, &app, -1),
                            ("tally-next", _) | ("tally-down", true) => step(&session, &lib, &proxy, &app, 1),
                            ("tally-up", false) => {
                                let _ = session.mpv.command(&["seek", "60"]);
                            }
                            ("tally-down", false) => {
                                let _ = session.mpv.command(&["seek", "-60"]);
                            }
                            ("tally-fit", _) => cycle_fit(&session),
                            ("tally-esc", _) => {
                                if session.mpv.get("fullscreen").as_deref() == Some("yes") {
                                    let _ = session.mpv.set("fullscreen", "no");
                                } else {
                                    let _ = session.mpv.command(&["quit"]);
                                }
                            }
                            _ => {}
                        }
                    }
                    Event::EndFile { eof, error } => {
                        let (live, last, retries) = {
                            let st = session.state.lock().unwrap();
                            (st.items[st.index].live_id.is_some(), st.index + 1 >= st.items.len(), st.retries)
                        };
                        if live || error.is_some() {
                            // a hiccup on the way: start the same thing again a few times
                            if retries < 3 {
                                session.state.lock().unwrap().retries += 1;
                                let _ = session.mpv.command(&["show-text", "Reconnecting…", "2500"]);
                                std::thread::sleep(Duration::from_millis(900 * (retries as u64 + 1)));
                                let _ = load_current(&session, &lib, &proxy);
                            } else {
                                let msg = error.unwrap_or_else(|| "the stream ended".into());
                                let _ = session.mpv.command(&["show-text", &format!("Could not play: {msg}"), "6000"]);
                                let _ = app.emit("hq-error", msg);
                            }
                        } else if eof && !last {
                            step(&session, &lib, &proxy, &app, 1);
                        } else if eof {
                            let _ = session.mpv.command(&["quit"]);
                        }
                    }
                    _ => {}
                }
            }
            *me.session.lock().unwrap() = None;
            drop(session);
            let _ = app.emit("hq-closed", ());
        });
        Ok(())
    }
}

fn step(s: &Arc<Session>, lib: &Arc<Library>, proxy: &Proxy, app: &AppHandle, d: i64) {
    let idx = {
        let mut st = s.state.lock().unwrap();
        let n = st.items.len() as i64;
        st.index = ((st.index as i64 + d).rem_euclid(n)) as usize;
        st.retries = 0;
        st.index
    };
    let _ = load_current(s, lib, proxy);
    let _ = app.emit("hq-index", idx);
}

fn cycle_fit(s: &Session) {
    let mut st = s.state.lock().unwrap();
    st.fit = (st.fit + 1) % 3;
    let (label, keep, pan) = match st.fit {
        0 => ("Fit", "yes", "0"),
        1 => ("Fill", "yes", "1"),
        _ => ("Stretch", "no", "0"),
    };
    let _ = s.mpv.set("keepaspect", keep);
    let _ = s.mpv.set("panscan", pan);
    let _ = s.mpv.command(&["show-text", label, "1200"]);
}

fn load_current(s: &Arc<Session>, lib: &Arc<Library>, proxy: &Proxy) -> Result<()> {
    let item = {
        let st = s.state.lock().unwrap();
        st.items[st.index].clone()
    };
    let url = resolve(&item, lib, proxy)?;
    proxy.new_session();
    let _ = s.mpv.set("force-media-title", &item.name);
    s.mpv.command(&["loadfile", &url, "replace"])?;
    let _ = s.mpv.command(&["show-text", &item.name, "3000"]);
    Ok(())
}

fn resolve(item: &Item, lib: &Arc<Library>, proxy: &Proxy) -> Result<String> {
    if let Some(id) = item.live_id {
        let store = lib.active_store()?.ok_or_else(|| Error::Account("no playlist".into()))?;
        let ch = store.channel_by_id(id)?.ok_or_else(|| Error::Account("channel not found".into()))?;
        return Ok(proxy.live_url_for(&ch.url) + "?lead=3");
    }
    let v = item.vod.as_ref().ok_or_else(|| Error::Account("nothing to play".into()))?;
    let client = lib.client()?.ok_or_else(|| Error::Account("no playlist".into()))?;
    let url = if v.kind == "movie" { client.movie_url(&v.id, &v.ext) } else { client.episode_url(&v.id, &v.ext) };
    Ok(proxy.url_for(&url))
}

/// Everything that makes the picture good, by tier. Set before the player starts.
fn configure(m: &Mpv, o: &Options) -> Result<()> {
    let mut opts: Vec<(&str, String)> = vec![
        // our own settings only: ignore any mpv.conf or scripts on the machine
        ("config", "no".into()),
        ("load-scripts", "no".into()),
        ("terminal", "no".into()),
        ("input-default-bindings", "yes".into()),
        ("input-vo-keyboard", "yes".into()),
        ("osc", "yes".into()),
        ("force-window", "yes".into()),
        ("idle", "yes".into()),
        ("keep-open", "no".into()),
        ("title", "Tally".into()),
        ("autofit", "70%".into()),
        ("fullscreen", if o.fullscreen { "yes" } else { "no" }.into()),
        ("hwdec", "auto-safe".into()),
        ("volume-max", "130".into()),
        ("cache", "yes".into()),
        ("cache-secs", o.buffer_secs.clamp(5, 120).to_string()),
        ("demuxer-max-bytes", "96MiB".into()),
        ("demuxer-readahead-secs", o.buffer_secs.clamp(5, 120).to_string()),
        ("network-timeout", "20".into()),
        // the relay hands over a live transport stream: look at it briefly, then start
        ("demuxer-lavf-analyzeduration", "2".into()),
        ("demuxer-lavf-probesize", "1500000".into()),
        ("audio-channels", "auto-safe".into()),
        ("screenshot-directory", "~~desktop/".into()),
    ];
    match o.quality.as_str() {
        "standard" => {
            opts.push(("vo", "gpu".into()));
            opts.push(("scale", "bilinear".into()));
            opts.push(("cscale", "bilinear".into()));
        }
        "ultra" => {
            opts.extend([
                ("vo", "gpu-next".into()), ("gpu-api", "d3d11".into()),
                ("scale", "ewa_lanczossharp".into()), ("cscale", "ewa_lanczossharp".into()), ("dscale", "ewa_lanczos".into()),
                ("correct-downscaling", "yes".into()), ("linear-downscaling", "yes".into()), ("sigmoid-upscaling", "yes".into()),
                ("deband", "yes".into()), ("deband-iterations", "2".into()), ("deband-grain", "24".into()),
                ("dither-depth", "auto".into()),
                ("video-sync", "display-resample".into()), ("interpolation", "yes".into()), ("tscale", "oversample".into()),
                ("hdr-compute-peak", "yes".into()), ("tone-mapping", "bt.2390".into()), ("target-colorspace-hint", "yes".into()),
            ]);
        }
        _ => {
            // high (default)
            opts.extend([
                ("vo", "gpu-next".into()), ("gpu-api", "d3d11".into()),
                ("scale", "ewa_lanczossharp".into()), ("cscale", "spline36".into()), ("dscale", "mitchell".into()),
                ("correct-downscaling", "yes".into()), ("sigmoid-upscaling", "yes".into()),
                ("deband", "yes".into()), ("dither-depth", "auto".into()),
                ("video-sync", "display-resample".into()), ("interpolation", "yes".into()), ("tscale", "oversample".into()),
            ]);
        }
    }
    for (k, v) in opts {
        // an option this build of the player does not know must not stop playback
        let _ = m.option(k, &v);
    }
    Ok(())
}
