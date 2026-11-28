//! Xtream Codes client: account check, playlist and guide download.
//!
//! Both downloads are streamed straight into the existing parsers, so a huge
//! playlist or guide never sits in memory as one blob.
//!
//! Credentials: they live only inside [`XtreamClient`]. `Debug` hides the password,
//! and every error message has the username and password scrubbed, because HTTP
//! libraries like to include the full request URL in their errors.

use crate::error::{Error, Result};
use crate::m3u::parse_m3u;
use crate::model::{Channel, Programme, SeriesDetail, Episode, Title};
use crate::xmltv::parse_xmltv;
use serde_json::Value;
use std::fmt;
use std::io::BufReader;
use std::time::Duration;

const USER_AGENTS: [&str; 5] = [
    "IPTVSmartersPro",
    "okhttp/4.12.0",
    "VLC/3.0.20 LibVLC/3.0.20",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36",
    "Tally/0.1",
];

#[derive(Clone)]
pub struct XtreamClient {
    base: String,
    user: String,
    pass: String,
    /// Tried in order. IPv4 first, because many hosts publish an IPv6 address that never answers
    /// and a browser's happy-eyeballs fallback is not something plain sockets do.
    agents: Vec<(ureq::config::IpFamily, ureq::Agent)>,
    /// Same combinations, but willing to wait for a big list (time to first byte up to 3 minutes).
    patient: Vec<ureq::Agent>,
    /// Index of the combination that last answered; used first, with the patient agent.
    working: std::sync::Arc<std::sync::Mutex<Option<usize>>>,
}

impl fmt::Debug for XtreamClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("XtreamClient")
            .field("base", &self.base)
            .field("user", &"***")
            .field("pass", &"***")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountInfo {
    /// Provider's own wording, usually "Active".
    pub status: String,
    /// Subscription end as Unix seconds, if the provider reports one.
    pub expires_at: Option<i64>,
    pub is_trial: bool,
    pub max_connections: Option<u32>,
    pub active_connections: Option<u32>,
    /// Stream container formats the account allows, e.g. `["m3u8", "ts"]`.
    pub formats: Vec<String>,
}

/// Xtream panels send ids as numbers or as strings, depending on the panel.
fn num_f(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(0.0),
        Value::String(s) => s.trim().parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.trim().to_string(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

fn first_text(v: &Value) -> String {
    match v {
        Value::Array(a) => a.first().map(text).unwrap_or_default(),
        other => text(other),
    }
}

fn id_str(v: &Value) -> Option<String> {
    match v {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

impl XtreamClient {
    pub fn new(base_url: &str, username: &str, password: &str) -> Self {
        let mut base = base_url.trim().trim_end_matches('/').to_string();
        if !base.contains("://") {
            base = format!("http://{base}");
        }
        let make = |family: ureq::config::IpFamily, ua: &str, recv: u64| -> ureq::Agent {
            ureq::Agent::config_builder()
                .ip_family(family)
                .user_agent(ua)
                .timeout_connect(Some(Duration::from_secs(20)))
                .timeout_recv_response(Some(Duration::from_secs(recv)))
                .build()
                .into()
        };
        // Outer: IPv4 first (many hosts publish an IPv6 address that never answers). Inner: some
        // providers refuse or silently ignore clients they do not recognise, so try player-style names.
        let mut agents = Vec::new();
        let mut patient = Vec::new();
        for family in [ureq::config::IpFamily::Ipv4Only, ureq::config::IpFamily::Any] {
            for (n, ua) in USER_AGENTS.iter().enumerate() {
                // The first name gets a long wait: some panels need a minute to answer the first call.
                agents.push((family, make(family, ua, if n == 0 { 180 } else { 30 })));
                patient.push(make(family, ua, 180));
            }
        }
        Self { base, user: username.to_string(), pass: password.to_string(), agents, patient, working: Default::default() }
    }

    fn url(&self, path: &str, extra: &str) -> String {
        format!(
            "{}/{}?username={}&password={}{}",
            self.base,
            path,
            encode(&self.user),
            encode(&self.pass),
            extra
        )
    }

    /// Remove the credentials from any text before it can reach a log or the UI.
    fn scrub(&self, msg: &str) -> String {
        let mut s = msg.to_string();
        for secret in [&self.pass, &self.user] {
            if secret.is_empty() {
                continue;
            }
            s = s.replace(&encode(secret), "***").replace(secret.as_str(), "***");
        }
        s
    }

    fn get(&self, url: &str) -> Result<ureq::Body> {
        // 1. The combination that worked before, allowed to be slow.
        let known = *self.working.lock().unwrap();
        if let Some(i) = known {
            match self.patient[i].get(url).call() {
                Ok(resp) => return Ok(resp.into_body()),
                Err(ureq::Error::StatusCode(code)) if code != 403 && code != 404 => {
                    return Err(Error::Http(format!("http status: {code}")));
                }
                Err(ureq::Error::Timeout(ureq::Timeout::RecvResponse)) => {
                    return Err(Error::Http("the server connected but took too long to answer. Try again in a minute.".into()));
                }
                Err(_) => *self.working.lock().unwrap() = None,
            }
        }
        // 2. Probe: short waits, every client name. A silent timeout usually means "unknown client".
        let mut last = String::new();
        let mut status: Option<u16> = None;
        let mut connected = false;
        let mut skip_family = None;
        for (i, (family, agent)) in self.agents.iter().enumerate() {
            if skip_family == Some(*family) {
                continue;
            }
            if *family == ureq::config::IpFamily::Any && connected {
                break;
            }
            match agent.get(url).call() {
                Ok(resp) => {
                    *self.working.lock().unwrap() = Some(i);
                    return Ok(resp.into_body());
                }
                Err(ureq::Error::StatusCode(code)) => {
                    connected = true;
                    if code != 403 && code != 404 {
                        return Err(Error::Http(format!("http status: {code}")));
                    }
                    status = Some(code);
                }
                Err(ureq::Error::Timeout(ureq::Timeout::RecvResponse)) => {
                    connected = true;
                    last = "the server connected but did not answer".into();
                }
                Err(e) => {
                    last = self.scrub(&e.to_string());
                    skip_family = Some(*family);
                }
            }
        }
        if let Some(code) = status {
            return Err(Error::Http(format!(
                "http status: {code}. The server answered but has no Xtream API at this address. Check the address (no /c or /get.php) and the port."
            )));
        }
        Err(Error::Http(format!(
            "could not reach the server ({last}). Check the address, and that this computer can open it."
        )))
    }

    /// Log in and read subscription details. Fails if the provider rejects the login.
    pub fn account(&self) -> Result<AccountInfo> {
        let mut body = self.get(&self.url("player_api.php", ""))?;
        let text = body
            .with_config()
            .limit(u64::MAX)
            .read_to_string()
            .map_err(|e| Error::Http(self.scrub(&e.to_string())))?;
        let v: Value = serde_json::from_str(&text)
            .map_err(|_| Error::Account("the server did not return account data".into()))?;
        parse_account(&v)
    }

    /// Stream the live playlist into `sink`. Returns how many channels arrived.
    pub fn playlist(&self, mut sink: impl FnMut(Channel)) -> Result<usize> {
        // The JSON API is on every panel; `get.php` is often switched off (404).
        match self.playlist_json(&mut sink) {
            Ok(n) if n > 0 => Ok(n),
            first => {
                // get.php is only a fallback; some providers answer it with a one-channel notice
                // video, which must never replace a real failure.
                let mut got = Vec::new();
                match self.playlist_m3u(|c| got.push(c)) {
                    Ok(n) if n > 1 => {
                        got.into_iter().for_each(&mut sink);
                        Ok(n)
                    }
                    _ => match first {
                        Err(e) => Err(e),
                        Ok(_) => Err(Error::Account("the provider returned no channels for this account".into())),
                    },
                }
            }
        }
    }

    fn json(&self, action: &str) -> Result<Value> {
        let mut body = self.get(&self.url("player_api.php", &format!("&action={action}")))?;
        // Real catalogues are tens of megabytes; the default 10 MB cap made big lists fail.
        let text = body
            .with_config()
            .limit(u64::MAX)
            .read_to_string()
            .map_err(|e| Error::Http(self.scrub(&e.to_string())))?;
        serde_json::from_str(&text).map_err(|_| Error::Account(format!("unexpected reply to {action}")))
    }

    /// Movies (`kind` = `movie`) or series (`series`) with their categories.
    pub fn titles(&self, kind: &str, mut sink: impl FnMut(Title)) -> Result<usize> {
        let (cats, list) = if kind == "movie" { ("get_vod_categories", "get_vod_streams") } else { ("get_series_categories", "get_series") };
        let mut names = std::collections::HashMap::new();
        if let Ok(Value::Array(cs)) = self.json(cats) {
            for c in cs {
                if let (Some(id), Some(name)) = (id_str(&c["category_id"]), c["category_name"].as_str()) {
                    names.insert(id, name.trim().to_string());
                }
            }
        }
        let items = match self.json(list)? {
            Value::Array(a) => a,
            _ => return Ok(0),
        };
        let mut n = 0;
        for it in &items {
            let idkey = if kind == "movie" { "stream_id" } else { "series_id" };
            let (Some(id), Some(name)) = (id_str(&it[idkey]), it["name"].as_str()) else { continue };
            let poster = if kind == "movie" { text(&it["stream_icon"]) } else { first_text(&it["cover"]) };
            let added = if kind == "movie" { num_f(&it["added"]) } else { num_f(&it["last_modified"]) } as i64;
            let year = {
                let y = text(&it["year"]);
                if y.is_empty() { text(&it["releaseDate"]).chars().take(4).collect() } else { y }
            };
            sink(Title {
                kind: kind.to_string(),
                ext_id: id,
                name: name.trim().to_string(),
                group: id_str(&it["category_id"]).and_then(|c| names.get(&c).cloned()).unwrap_or_default(),
                poster,
                plot: text(&it["plot"]),
                rating: if it.get("rating_5based").is_some() && num_f(&it["rating"]) == 0.0 { num_f(&it["rating_5based"]) * 2.0 } else { num_f(&it["rating"]) },
                year,
                added,
                ext: if kind == "movie" { text(&it["container_extension"]) } else { String::new() },
            });
            n += 1;
        }
        Ok(n)
    }

    /// Seasons and episodes of one series.
    pub fn series_detail(&self, series_id: &str) -> Result<SeriesDetail> {
        let v = self.json(&format!("get_series_info&series_id={}", encode(series_id)))?;
        let info = &v["info"];
        let mut out = SeriesDetail { name: text(&info["name"]), plot: text(&info["plot"]), poster: first_text(&info["cover"]), episodes: Vec::new() };
        if let Value::Object(seasons) = &v["episodes"] {
            for (sn, list) in seasons {
                let Value::Array(list) = list else { continue };
                for e in list {
                    let Some(id) = id_str(&e["id"]) else { continue };
                    out.episodes.push(Episode {
                        id,
                        season: sn.parse().unwrap_or_else(|_| num_f(&e["season"]) as u32),
                        number: num_f(&e["episode_num"]) as u32,
                        title: text(&e["title"]),
                        plot: text(&e["info"]["plot"]),
                        duration: text(&e["info"]["duration"]),
                        image: text(&e["info"]["movie_image"]),
                        ext: { let x = text(&e["container_extension"]); if x.is_empty() { "mp4".into() } else { x } },
                    });
                }
            }
        }
        out.episodes.sort_by_key(|e| (e.season, e.number));
        Ok(out)
    }

    pub fn movie_url(&self, id: &str, ext: &str) -> String {
        format!("{}/movie/{}/{}/{}.{}", self.base, encode(&self.user), encode(&self.pass), id, if ext.is_empty() { "mp4" } else { ext })
    }

    pub fn episode_url(&self, id: &str, ext: &str) -> String {
        format!("{}/series/{}/{}/{}.{}", self.base, encode(&self.user), encode(&self.pass), id, if ext.is_empty() { "mp4" } else { ext })
    }

    fn playlist_json(&self, sink: &mut impl FnMut(Channel)) -> Result<usize> {
        let mut names = std::collections::HashMap::new();
        if let Ok(Value::Array(cats)) = self.json("get_live_categories") {
            for c in cats {
                if let (Some(id), Some(name)) = (id_str(&c["category_id"]), c["category_name"].as_str()) {
                    names.insert(id, name.to_string());
                }
            }
        }
        let streams = match self.json("get_live_streams")? {
            Value::Array(a) => a,
            _ => return Ok(0),
        };
        // HLS first when the plan allows it, because that is what most providers serve reliably.
        let ext = match self.account().map(|a| a.formats) {
            Ok(f) if f.iter().any(|x| x == "m3u8") => "m3u8",
            _ => "ts",
        };
        let mut n = 0;
        for (i, s) in streams.iter().enumerate() {
            let (Some(id), Some(name)) = (id_str(&s["stream_id"]), s["name"].as_str()) else { continue };
            sink(Channel {
                key: format!("x:{id}"),
                tvg_id: s["epg_channel_id"].as_str().unwrap_or("").to_string(),
                name: name.trim().to_string(),
                group: id_str(&s["category_id"]).and_then(|c| names.get(&c).cloned()).unwrap_or_default(),
                logo: s["stream_icon"].as_str().unwrap_or("").to_string(),
                url: format!("{}/live/{}/{}/{}.{}", self.base, encode(&self.user), encode(&self.pass), id, ext),
                number: id_str(&s["num"]).and_then(|x| x.parse().ok()).or(Some(i as u32 + 1)),
            });
            n += 1;
        }
        Ok(n)
    }

    fn playlist_m3u(&self, sink: impl FnMut(Channel)) -> Result<usize> {
        let mut body = self.get(&self.url("get.php", "&type=m3u_plus&output=ts"))?;
        let reader = BufReader::new(body.with_config().limit(u64::MAX).reader());
        parse_m3u(reader, sink).map_err(|e| Error::Http(self.scrub(&e.to_string())))
    }

    pub fn guide(&self, sink: impl FnMut(Programme)) -> Result<usize> {
        let mut body = self.get(&self.url("xmltv.php", ""))?;
        let reader = BufReader::new(body.with_config().limit(u64::MAX).reader());
        parse_xmltv(reader, sink).map_err(|e| match e {
            Error::Xml(m) => Error::Xml(self.scrub(&m)),
            other => other,
        })
    }
}

fn parse_account(v: &Value) -> Result<AccountInfo> {
    let info = v
        .get("user_info")
        .ok_or_else(|| Error::Account("the server did not return account data".into()))?;
    if num(info.get("auth")).unwrap_or(0) != 1 {
        return Err(Error::Account("login rejected (check username and password)".into()));
    }
    Ok(AccountInfo {
        status: info.get("status").and_then(Value::as_str).unwrap_or("").to_string(),
        expires_at: num(info.get("exp_date")).filter(|n| *n > 0),
        is_trial: num(info.get("is_trial")).unwrap_or(0) == 1,
        max_connections: num(info.get("max_connections")).and_then(|n| u32::try_from(n).ok()),
        active_connections: num(info.get("active_cons")).and_then(|n| u32::try_from(n).ok()),
        formats: info
            .get("allowed_output_formats")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
            .unwrap_or_default(),
    })
}

/// Providers send numbers as JSON numbers or as strings, inconsistently.
fn num(v: Option<&Value>) -> Option<i64> {
    match v? {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// Percent-encode everything outside the URL "unreserved" set.
fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// Minimal one-shot HTTP server for tests: answers `n` requests, routing on the path.
    fn serve(n: usize, wrong_login: bool) -> String {
        serve_with(n, wrong_login, true)
    }

    fn serve_with(n: usize, wrong_login: bool, json_lists: bool) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for _ in 0..n {
                let (mut s, _) = listener.accept().unwrap();
                let mut buf = [0u8; 4096];
                let len = s.read(&mut buf).unwrap();
                let req = String::from_utf8_lossy(&buf[..len]).to_string();
                let line = req.lines().next().unwrap_or("").to_string();
                let body = if json_lists && line.contains("action=get_live_categories") {
                    r#"[{"category_id":"7","category_name":"News"},{"category_id":8,"category_name":"أخبار"}]"#.to_string()
                } else if json_lists && line.contains("action=get_live_streams") {
                    r#"[{"num":1,"name":"Harbor News 24","stream_id":101,"epg_channel_id":"hn","category_id":"7","stream_icon":""},
                        {"num":"2","name":"المرفأ نيوز","stream_id":"102","epg_channel_id":null,"category_id":8}]"#.to_string()
                } else if !json_lists && line.contains("action=") {
                    "<html>not found</html>".to_string()
                } else if line.contains("player_api.php") {
                    if wrong_login || !line.contains("password=p%40ss") {
                        r#"{"user_info":{"auth":0}}"#.to_string()
                    } else {
                        r#"{"user_info":{"username":"u","auth":1,"status":"Active","exp_date":"1807660800",
                            "is_trial":"0","active_cons":"0","max_connections":"2",
                            "allowed_output_formats":["m3u8","ts"]},"server_info":{}}"#
                            .to_string()
                    }
                } else if line.contains("get.php") {
                    "#EXTM3U\n#EXTINF:-1 tvg-id=\"hn\" group-title=\"News\",Harbor News 24\nhttp://s/live/1.ts\n\
                     #EXTINF:-1 tvg-id=\"ar\" group-title=\"أخبار\",المرفأ نيوز\nhttp://s/live/2.ts\n"
                        .to_string()
                } else if line.contains("xmltv.php") {
                    "<tv><programme start=\"20261121200000 +0000\" stop=\"20261121210000 +0000\" channel=\"hn\">\
                     <title>Evening Bulletin</title></programme></tv>"
                        .to_string()
                } else {
                    String::new()
                };
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                s.write_all(resp.as_bytes()).unwrap();
            }
        });
        format!("127.0.0.1:{}", addr.port())
    }

    #[test]
    fn account_playlist_and_guide_round_trip() {
        let host = serve(5, false);
        // the password has a character that must be percent-encoded
        let c = XtreamClient::new(&host, "u", "p@ss");

        let acct = c.account().unwrap();
        assert_eq!(acct.status, "Active");
        assert_eq!(acct.expires_at, Some(1_807_660_800));
        assert_eq!(acct.max_connections, Some(2));
        assert!(!acct.is_trial);
        assert_eq!(acct.formats, vec!["m3u8", "ts"]);

        let mut chans = Vec::new();
        assert_eq!(c.playlist(|ch| chans.push(ch)).unwrap(), 2);
        assert_eq!(chans[1].name, "المرفأ نيوز");
        assert_eq!(chans[0].group, "News");
        assert_eq!(chans[1].group, "أخبار");
        assert_eq!(chans[0].number, Some(1));
        // HLS is allowed on this plan, and the password is encoded in the stream address
        assert!(chans[0].url.ends_with("/live/u/p%40ss/101.m3u8"), "{}", chans[0].url);

        let mut progs = Vec::new();
        assert_eq!(c.guide(|p| progs.push(p)).unwrap(), 1);
        assert_eq!(progs[0].title, "Evening Bulletin");
    }

    #[test]
    fn falls_back_to_m3u_when_json_lists_are_missing() {
        // get_live_categories, get_live_streams, then get.php
        let host = serve_with(3, false, false);
        let c = XtreamClient::new(&host, "u", "p@ss");
        let mut chans = Vec::new();
        assert_eq!(c.playlist(|ch| chans.push(ch)).unwrap(), 2);
        assert_eq!(chans[0].name, "Harbor News 24");
    }

    #[test]
    fn rejected_login_is_a_clear_error() {
        let host = serve(1, true);
        let err = XtreamClient::new(&host, "u", "wrong").account().unwrap_err();
        assert!(matches!(err, Error::Account(_)));
    }

    #[test]
    fn secrets_never_appear_in_debug_or_errors() {
        let c = XtreamClient::new("127.0.0.1:1", "user12345", "pa$$word99");
        assert!(!format!("{c:?}").contains("pa$$word99"));
        assert!(!format!("{c:?}").contains("user12345"));
        // connection refused: the library error would normally echo the URL
        let msg = c.account().unwrap_err().to_string();
        assert!(!msg.contains("pa$$word99") && !msg.contains("pa%24%24word99"), "leaked: {msg}");
        assert!(!msg.contains("user12345"), "leaked: {msg}");
    }

    #[test]
    fn encoding() {
        assert_eq!(encode("p@ss w/rd"), "p%40ss%20w%2Frd");
        assert_eq!(encode("abc-_.~123"), "abc-_.~123");
    }
}
