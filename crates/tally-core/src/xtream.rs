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
use crate::model::{Channel, Programme};
use crate::xmltv::parse_xmltv;
use serde_json::Value;
use std::fmt;
use std::io::BufReader;
use std::time::Duration;

#[derive(Clone)]
pub struct XtreamClient {
    base: String,
    user: String,
    pass: String,
    /// Tried in order. IPv4 first, because many hosts publish an IPv6 address that never answers
    /// and a browser's happy-eyeballs fallback is not something plain sockets do.
    agents: Vec<ureq::Agent>,
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

impl XtreamClient {
    pub fn new(base_url: &str, username: &str, password: &str) -> Self {
        let mut base = base_url.trim().trim_end_matches('/').to_string();
        if !base.contains("://") {
            base = format!("http://{base}");
        }
        let make = |family: ureq::config::IpFamily| -> ureq::Agent {
            ureq::Agent::config_builder()
                .ip_family(family)
                .user_agent("Tally/0.1")
                .timeout_connect(Some(Duration::from_secs(8)))
                .timeout_recv_response(Some(Duration::from_secs(30)))
                .build()
                .into()
        };
        let agents = vec![make(ureq::config::IpFamily::Ipv4Only), make(ureq::config::IpFamily::Any)];
        Self { base, user: username.to_string(), pass: password.to_string(), agents }
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
        let mut last = String::new();
        for agent in &self.agents {
            match agent.get(url).call() {
                Ok(resp) => return Ok(resp.into_body()),
                // The server answered (wrong login, 404...): another network path will not change that.
                Err(e @ ureq::Error::StatusCode(_)) => return Err(Error::Http(self.scrub(&e.to_string()))),
                Err(e) => last = self.scrub(&e.to_string()),
            }
        }
        Err(Error::Http(format!(
            "could not reach the server ({last}). Check the address, and that this computer can open it."
        )))
    }

    /// Log in and read subscription details. Fails if the provider rejects the login.
    pub fn account(&self) -> Result<AccountInfo> {
        let mut body = self.get(&self.url("player_api.php", ""))?;
        let text = body
            .read_to_string()
            .map_err(|e| Error::Http(self.scrub(&e.to_string())))?;
        let v: Value = serde_json::from_str(&text)
            .map_err(|_| Error::Account("the server did not return account data".into()))?;
        parse_account(&v)
    }

    /// Stream the live playlist into `sink`. Returns how many channels arrived.
    pub fn playlist(&self, sink: impl FnMut(Channel)) -> Result<usize> {
        let mut body = self.get(&self.url("get.php", "&type=m3u_plus&output=ts"))?;
        let reader = BufReader::new(body.with_config().limit(u64::MAX).reader());
        parse_m3u(reader, sink).map_err(|e| Error::Http(self.scrub(&e.to_string())))
    }

    /// Stream the XMLTV guide into `sink`. Returns how many programmes arrived.
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
        expires_at: num(info.get("exp_date")),
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
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for _ in 0..n {
                let (mut s, _) = listener.accept().unwrap();
                let mut buf = [0u8; 4096];
                let len = s.read(&mut buf).unwrap();
                let req = String::from_utf8_lossy(&buf[..len]).to_string();
                let line = req.lines().next().unwrap_or("").to_string();
                let body = if line.contains("player_api.php") {
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
        let host = serve(3, false);
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

        let mut progs = Vec::new();
        assert_eq!(c.guide(|p| progs.push(p)).unwrap(), 1);
        assert_eq!(progs[0].title, "Evening Bulletin");
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
