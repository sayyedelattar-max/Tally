//! The user's saved playlists: what the Playlists screen lists, edits and refreshes.
//!
//! Layout inside the app data folder:
//!
//! * `playlists.json`  names, server, username, subscription details (never the password)
//! * `playlist-<id>.db` one channel database per playlist
//!
//! Passwords go through the [`Secrets`] trait, which the desktop app backs with the operating
//! system's credential store. They are never written to the folder.

use crate::xtream::XtreamClient;
use crate::{Error, Result, Store};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Somewhere safe to keep one password per playlist id.
pub trait Secrets: Send + Sync {
    fn get(&self, id: &str) -> Option<String>;
    fn set(&self, id: &str, secret: &str) -> Result<()>;
    fn delete(&self, id: &str);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistMeta {
    pub id: String,
    pub name: String,
    pub host: String,
    pub user: String,
    pub status: String,
    pub expires_at: Option<i64>,
    pub trial: bool,
    pub max_connections: Option<u32>,
    pub active_connections: Option<u32>,
    pub channels: i64,
    pub updated_at: i64,
}

pub struct Library {
    dir: PathBuf,
    secrets: Box<dyn Secrets>,
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn meta_to_json(m: &PlaylistMeta) -> Value {
    json!({
        "id": m.id, "name": m.name, "host": m.host, "user": m.user, "status": m.status,
        "expires_at": m.expires_at, "trial": m.trial, "max_connections": m.max_connections,
        "active_connections": m.active_connections, "channels": m.channels, "updated_at": m.updated_at,
    })
}

fn meta_from_json(v: &Value) -> Option<PlaylistMeta> {
    let s = |k: &str| v[k].as_str().map(str::to_string);
    Some(PlaylistMeta {
        id: s("id")?,
        name: s("name").unwrap_or_default(),
        host: s("host")?,
        user: s("user")?,
        status: s("status").unwrap_or_default(),
        expires_at: v["expires_at"].as_i64(),
        trial: v["trial"].as_bool().unwrap_or(false),
        max_connections: v["max_connections"].as_u64().map(|x| x as u32),
        active_connections: v["active_connections"].as_u64().map(|x| x as u32),
        channels: v["channels"].as_i64().unwrap_or(0),
        updated_at: v["updated_at"].as_i64().unwrap_or(0),
    })
}

impl Library {
    pub fn new(dir: impl Into<PathBuf>, secrets: Box<dyn Secrets>) -> Self {
        Self { dir: dir.into(), secrets }
    }

    fn index_path(&self) -> PathBuf {
        self.dir.join("playlists.json")
    }

    fn db_path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("playlist-{id}.db"))
    }

    fn read(&self) -> Result<(Option<String>, Vec<PlaylistMeta>)> {
        let text = match std::fs::read_to_string(self.index_path()) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok((None, Vec::new())),
            Err(e) => return Err(e.into()),
        };
        let v: Value = serde_json::from_str(&text).map_err(|_| Error::Account("playlists.json is damaged".into()))?;
        let items = v["items"].as_array().map(|a| a.iter().filter_map(meta_from_json).collect()).unwrap_or_default();
        Ok((v["active"].as_str().map(str::to_string), items))
    }

    fn write(&self, active: &Option<String>, items: &[PlaylistMeta]) -> Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        let v = json!({ "active": active, "items": items.iter().map(meta_to_json).collect::<Vec<_>>() });
        let tmp = self.dir.join("playlists.json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(&v).unwrap_or_default())?;
        std::fs::rename(&tmp, self.index_path())?;
        Ok(())
    }

    /// The active playlist id and every saved playlist, in the order they were added.
    pub fn list(&self) -> Result<(Option<String>, Vec<PlaylistMeta>)> {
        let (mut active, items) = self.read()?;
        if active.as_ref().is_none_or(|a| !items.iter().any(|m| &m.id == a)) {
            active = items.first().map(|m| m.id.clone());
        }
        Ok((active, items))
    }

    /// Add a playlist (`id` = `None`) or change one. The server is contacted first, and nothing is
    /// saved if the login or the download fails. An empty password keeps the stored one.
    pub fn save(&self, id: Option<&str>, name: &str, host: &str, user: &str, pass: Option<&str>) -> Result<PlaylistMeta> {
        let (host, user) = (host.trim(), user.trim());
        if host.is_empty() || user.is_empty() {
            return Err(Error::Account("the server address and the username are required".into()));
        }
        let (active, mut items) = self.read()?;
        let existing = id.and_then(|i| items.iter().find(|m| m.id == i)).cloned();
        if id.is_some() && existing.is_none() {
            return Err(Error::Account("that playlist no longer exists".into()));
        }
        let id = existing.as_ref().map(|m| m.id.clone()).unwrap_or_else(|| format!("{:x}", now() * 1000 + (items.len() as i64)));
        let pass = match pass.filter(|p| !p.is_empty()) {
            Some(p) => p.to_string(),
            None => self.secrets.get(&id).ok_or_else(|| Error::Account("enter the password".into()))?,
        };

        let client = XtreamClient::new(host, user, &pass);
        let acct = client.account()?;
        let mut list = Vec::new();
        client.playlist(|c| list.push(c))?;

        std::fs::create_dir_all(&self.dir)?;
        // A different account means a different catalogue: start clean.
        if existing.as_ref().is_some_and(|m| m.host != host || m.user != user) {
            self.remove_db(&id);
        }
        let mut store = Store::open(&self.db_path(&id))?;
        let n = store.import_channels(list)?;
        self.secrets.set(&id, &pass)?;

        let meta = PlaylistMeta {
            id: id.clone(),
            name: if name.trim().is_empty() { host.to_string() } else { name.trim().to_string() },
            host: host.to_string(),
            user: user.to_string(),
            status: acct.status,
            expires_at: acct.expires_at,
            trial: acct.is_trial,
            max_connections: acct.max_connections,
            active_connections: acct.active_connections,
            channels: n as i64,
            updated_at: now(),
        };
        match items.iter_mut().find(|m| m.id == id) {
            Some(slot) => *slot = meta.clone(),
            None => items.push(meta.clone()),
        }
        let active = active.or_else(|| Some(id));
        self.write(&active, &items)?;
        Ok(meta)
    }

    /// Download the channels again, keeping the saved name, server and password.
    pub fn refresh(&self, id: &str) -> Result<PlaylistMeta> {
        let (_, items) = self.read()?;
        let m = items.iter().find(|m| m.id == id).ok_or_else(|| Error::Account("that playlist no longer exists".into()))?;
        self.save(Some(id), &m.name.clone(), &m.host.clone(), &m.user.clone(), None)
    }

    pub fn remove(&self, id: &str) -> Result<()> {
        let (active, mut items) = self.read()?;
        items.retain(|m| m.id != id);
        self.remove_db(id);
        self.secrets.delete(id);
        let active = if active.as_deref() == Some(id) { items.first().map(|m| m.id.clone()) } else { active };
        self.write(&active, &items)
    }

    pub fn activate(&self, id: &str) -> Result<()> {
        let (_, items) = self.read()?;
        if !items.iter().any(|m| m.id == id) {
            return Err(Error::Account("that playlist no longer exists".into()));
        }
        self.write(&Some(id.to_string()), &items)
    }

    /// The channel database of the active playlist, if there is one.
    pub fn active_store(&self) -> Result<Option<Store>> {
        match self.list()?.0 {
            Some(id) => Ok(Some(Store::open(&self.db_path(&id))?)),
            None => Ok(None),
        }
    }

    fn remove_db(&self, id: &str) {
        let base = self.db_path(id);
        for suffix in ["", "-wal", "-shm"] {
            let mut p = base.clone().into_os_string();
            p.push(suffix);
            let _ = std::fs::remove_file(Path::new(&p));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    #[derive(Default, Clone)]
    struct Mem(Arc<Mutex<HashMap<String, String>>>);
    impl Secrets for Mem {
        fn get(&self, id: &str) -> Option<String> {
            self.0.lock().unwrap().get(id).cloned()
        }
        fn set(&self, id: &str, s: &str) -> Result<()> {
            self.0.lock().unwrap().insert(id.into(), s.into());
            Ok(())
        }
        fn delete(&self, id: &str) {
            self.0.lock().unwrap().remove(id);
        }
    }

    /// Answers any number of requests like a small Xtream panel.
    fn panel() -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for conn in l.incoming() {
                let Ok(mut s) = conn else { break };
                let mut buf = [0u8; 4096];
                let n = s.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let line = req.lines().next().unwrap_or("").to_string();
                let body = if !line.contains("password=secret") {
                    r#"{"user_info":{"auth":0}}"#
                } else if line.contains("action=get_live_categories") {
                    r#"[{"category_id":"1","category_name":"News"}]"#
                } else if line.contains("action=get_live_streams") {
                    r#"[{"num":1,"name":"Harbor News","stream_id":11,"category_id":"1"},{"num":2,"name":"Delta","stream_id":12,"category_id":"1"}]"#
                } else {
                    r#"{"user_info":{"auth":1,"status":"Active","exp_date":"1807812511","is_trial":"0","max_connections":"1","allowed_output_formats":["m3u8"]}}"#
                };
                let r = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
                let _ = s.write_all(r.as_bytes());
            }
        });
        format!("127.0.0.1:{port}")
    }

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tally-lib-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn add_list_edit_refresh_remove() {
        let host = panel();
        let dir = tmp("flow");
        let secrets = Mem::default();
        let lib = Library::new(&dir, Box::new(secrets.clone()));
        assert_eq!(lib.list().unwrap().1.len(), 0);

        let a = lib.save(None, "Home", &host, "u", Some("secret")).unwrap();
        assert_eq!((a.name.as_str(), a.channels, a.status.as_str()), ("Home", 2, "Active"));
        let (active, items) = lib.list().unwrap();
        assert_eq!(active.as_deref(), Some(a.id.as_str()));
        assert_eq!(items.len(), 1);

        // the password lives in the secret store, never in the folder
        assert_eq!(secrets.get(&a.id).as_deref(), Some("secret"));
        let index = std::fs::read_to_string(dir.join("playlists.json")).unwrap();
        assert!(!index.contains("secret"));

        // rename without retyping the password
        let b = lib.save(Some(&a.id), "Living room", &host, "u", None).unwrap();
        assert_eq!(b.id, a.id);
        assert_eq!(b.name, "Living room");
        assert_eq!(lib.list().unwrap().1.len(), 1);

        assert_eq!(lib.refresh(&a.id).unwrap().channels, 2);
        let store = lib.active_store().unwrap().unwrap();
        assert_eq!(store.groups().unwrap(), vec![("News".to_string(), 2)]);

        // a second playlist, then switch and remove
        let c = lib.save(None, "Second", &host, "u2", Some("secret")).unwrap();
        lib.activate(&c.id).unwrap();
        assert_eq!(lib.list().unwrap().0.as_deref(), Some(c.id.as_str()));
        lib.remove(&c.id).unwrap();
        assert_eq!(lib.list().unwrap().0.as_deref(), Some(a.id.as_str()));
        assert!(secrets.get(&c.id).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn wrong_password_saves_nothing() {
        let host = panel();
        let dir = tmp("bad");
        let lib = Library::new(&dir, Box::new(Mem::default()));
        assert!(lib.save(None, "x", &host, "u", Some("nope")).is_err());
        assert_eq!(lib.list().unwrap().1.len(), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
