//! A tiny local web server that sits between the player and the IPTV provider.
//!
//! Why it exists: the web view that draws the player may not read pages from another site
//! (no CORS headers from IPTV panels), and the provider's address contains the account
//! password, which should never reach the page. The page only ever sees
//! `http://127.0.0.1:<port>/s/<secret>/<opaque>`; the server fetches the real address,
//! rewrites playlists (`.m3u8`) so every segment also goes through here, and streams the rest.
//!
//! It listens on the loopback interface only, and every request must carry a random secret
//! chosen at start-up.

use crate::{Error, Result};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::time::Duration;
use ureq::ResponseExt;
use url::Url;

const STREAM_UA: &str = "VLC/3.0.20 LibVLC/3.0.20";

#[derive(Clone)]
pub struct Proxy {
    port: u16,
    secret: Arc<str>,
}

fn stream_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .user_agent(STREAM_UA)
        .timeout_connect(Some(Duration::from_secs(8)))
        .timeout_recv_response(Some(Duration::from_secs(20)))
        .build()
        .into()
}

fn random_hex() -> String {
    let mut out = String::new();
    for i in 0..2u64 {
        let mut h = RandomState::new().build_hasher();
        h.write_u64(i);
        h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
        out.push_str(&format!("{:016x}", h.finish()));
    }
    out
}

impl Proxy {
    pub fn start() -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        let proxy = Proxy { port, secret: random_hex().into() };
        let me = proxy.clone();
        std::thread::spawn(move || {
            let agent = stream_agent();
            for conn in listener.incoming() {
                let Ok(stream) = conn else { continue };
                let (me, agent) = (me.clone(), agent.clone());
                std::thread::spawn(move || {
                    let _ = me.handle(stream, &agent);
                });
            }
        });
        Ok(proxy)
    }

    /// The address the player should open for an upstream address.
    pub fn url_for(&self, upstream: &str) -> String {
        format!("http://127.0.0.1:{}/s/{}/{}", self.port, self.secret, B64.encode(upstream))
    }

    fn handle(&self, mut stream: TcpStream, agent: &ureq::Agent) -> std::io::Result<()> {
        stream.set_read_timeout(Some(Duration::from_secs(15)))?;
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut first = String::new();
        reader.read_line(&mut first)?;
        let mut parts = first.split_whitespace();
        let (method, path) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
        let (method, path) = (method.to_string(), path.to_string());
        let mut range = None;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line)? == 0 || line.trim().is_empty() {
                break;
            }
            if let Some((k, v)) = line.split_once(':') {
                if k.eq_ignore_ascii_case("range") {
                    range = Some(v.trim().to_string());
                }
            }
        }
        if method == "OPTIONS" {
            return reply(&mut stream, 204, "No Content", &[], b"");
        }
        if method != "GET" && method != "HEAD" {
            return reply(&mut stream, 405, "Method Not Allowed", &[], b"");
        }
        let prefix = format!("/s/{}/", self.secret);
        let Some(token) = path.strip_prefix(prefix.as_str()) else {
            return reply(&mut stream, 403, "Forbidden", &[], b"");
        };
        let upstream = B64
            .decode(token.split(['?', '.']).next().unwrap_or(""))
            .ok()
            .and_then(|b| String::from_utf8(b).ok())
            .filter(|u| u.starts_with("http://") || u.starts_with("https://"));
        let Some(upstream) = upstream else {
            return reply(&mut stream, 400, "Bad Request", &[], b"bad address");
        };

        let mut req = agent.get(&upstream);
        if let Some(r) = &range {
            req = req.header("Range", r);
        }
        let resp = match req.call() {
            Ok(r) => r,
            Err(e) => {
                // Never echo the address: it contains the account password.
                let msg = format!("upstream error: {}", scrub(&e.to_string(), &upstream));
                return reply(&mut stream, 502, "Bad Gateway", &[], msg.as_bytes());
            }
        };
        let status = resp.status().as_u16();
        let final_uri = resp.get_uri().to_string();
        let hdrs: Vec<(String, String)> = resp.headers().iter().map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string())).collect();
        let header = |n: &str| hdrs.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone());
        let ctype = header("content-type").unwrap_or_default();
        let mut body = resp.into_body().into_reader();

        // Decide by what the provider actually sends, not by the address: some panels answer a
        // `.m3u8` request with a raw MPEG-TS stream, which must never be buffered as a "playlist".
        let mut first = vec![0u8; 4096];
        let got = read_some(&mut body, &mut first);
        first.truncate(got);
        let is_playlist = status == 200 && strip_bom(&first).trim_ascii_start().starts_with(b"#EXTM3U");
        if is_playlist {
            let mut rest = Vec::new();
            body.by_ref().take(8 * 1024 * 1024).read_to_end(&mut rest)?;
            let mut all = first;
            all.extend_from_slice(&rest);
            let text = String::from_utf8_lossy(&all).into_owned();
            let out = match Url::parse(&final_uri) {
                Ok(base) => rewrite_playlist(&text, &base, |u| self.url_for(u)),
                Err(_) => text,
            };
            return reply(&mut stream, 200, "OK", &[("Content-Type", "application/vnd.apple.mpegurl")], out.as_bytes());
        }
        let ctype = if first.first() == Some(&0x47) && !ctype.to_ascii_lowercase().starts_with("video/") { "video/mp2t".to_string() } else { ctype };

        let mut head = format!("HTTP/1.1 {status} {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n", if status == 206 { "Partial Content" } else if status == 200 { "OK" } else { "Upstream" });
        head.push_str(&format!("content-type: {ctype}\r\n"));
        for n in ["content-length", "content-range", "accept-ranges"] {
            if let Some(v) = header(n) {
                head.push_str(&format!("{n}: {v}\r\n"));
            }
        }
        head.push_str("\r\n");
        stream.write_all(head.as_bytes())?;
        if method == "HEAD" {
            return Ok(());
        }
        if stream.write_all(&first).is_err() {
            return Ok(());
        }
        let mut buf = [0u8; 64 * 1024];
        loop {
            let n = match body.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            if stream.write_all(&buf[..n]).is_err() {
                break; // the player moved on
            }
        }
        Ok(())
    }
}

fn read_some(r: &mut impl Read, buf: &mut [u8]) -> usize {
    let mut n = 0;
    while n < 16 {
        match r.read(&mut buf[n..]) {
            Ok(0) | Err(_) => break,
            Ok(k) => n += k,
        }
    }
    n
}

fn strip_bom(b: &[u8]) -> &[u8] {
    b.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(b)
}

/// What the provider returns for a stream address, described without revealing the address.
/// Shown to the user so a failure can be reported precisely.
pub fn diagnose(upstream: &str) -> String {
    use std::time::Instant;
    let agent = stream_agent();
    let mut out = Vec::new();
    let kind = |b: &[u8]| -> &'static str {
        let t = strip_bom(b).trim_ascii_start();
        if t.starts_with(b"#EXTM3U") { "HLS playlist" }
        else if b.first() == Some(&0x47) { "MPEG-TS stream" }
        else if t.starts_with(b"<") { "web page / error page" }
        else if b.is_empty() { "nothing" }
        else { "unknown data" }
    };
    let host = |u: &str| Url::parse(u).ok().and_then(|u| u.host_str().map(|h| format!("{h}{}", u.port().map(|p| format!(":{p}")).unwrap_or_default()))).unwrap_or_default();
    let t0 = Instant::now();
    let resp = match agent.get(upstream).call() {
        Ok(r) => r,
        Err(e) => return format!("request failed after {} ms: {}", t0.elapsed().as_millis(), scrub(&e.to_string(), upstream)),
    };
    let status = resp.status().as_u16();
    let final_uri = resp.get_uri().to_string();
    let ctype = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("-").to_string();
    out.push(format!("status {status}, {} ms to answer", t0.elapsed().as_millis()));
    out.push(format!("served by {}", host(&final_uri)));
    out.push(format!("content-type {ctype}"));
    let mut body = resp.into_body().into_reader();
    let mut first = vec![0u8; 4096];
    let t1 = Instant::now();
    let got = read_some(&mut body, &mut first);
    first.truncate(got);
    out.push(format!("first data after {} ms: {} ({} bytes read)", t1.elapsed().as_millis(), kind(&first), got));
    if kind(&first) == "HLS playlist" {
        let mut rest = Vec::new();
        let _ = body.by_ref().take(2 * 1024 * 1024).read_to_end(&mut rest);
        let mut all = first;
        all.extend_from_slice(&rest);
        let text = String::from_utf8_lossy(&all).into_owned();
        let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
        let segs: Vec<&str> = lines.iter().copied().filter(|l| !l.starts_with('#')).collect();
        out.push(format!("playlist: {} entries, {}", segs.len(), if text.contains("#EXT-X-ENDLIST") { "finished (not live)" } else { "live" }));
        if lines.iter().any(|l| l.starts_with("#EXT-X-STREAM-INF")) { out.push("playlist lists several qualities".into()); }
        if let (Some(first_seg), Ok(base)) = (segs.first(), Url::parse(&final_uri)) {
            if let Ok(u) = base.join(first_seg) {
                let t2 = Instant::now();
                match agent.get(u.as_str()).call() {
                    Ok(r) => {
                        let st = r.status().as_u16();
                        let ct = r.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("-").to_string();
                        let mut b = r.into_body().into_reader();
                        let mut f = vec![0u8; 4096];
                        let n = read_some(&mut b, &mut f);
                        f.truncate(n);
                        out.push(format!("first entry: status {st}, {} ms, {}, {ct}, served by {}", t2.elapsed().as_millis(), kind(&f), host(u.as_str())));
                    }
                    Err(e) => out.push(format!("first entry failed: {}", scrub(&e.to_string(), u.as_str()))),
                }
            }
        }
    }
    out.join("\n")
}

fn scrub(msg: &str, upstream: &str) -> String {
    let mut out = msg.replace(upstream, "<stream>");
    if let Ok(u) = Url::parse(upstream) {
        for seg in u.path_segments().into_iter().flatten().filter(|s| s.len() > 3) {
            out = out.replace(seg, "…");
        }
    }
    out
}

fn reply(stream: &mut TcpStream, code: u16, text: &str, extra: &[(&str, &str)], body: &[u8]) -> std::io::Result<()> {
    let mut head = format!(
        "HTTP/1.1 {code} {text}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: range\r\nAccess-Control-Allow-Methods: GET, HEAD, OPTIONS\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    for (k, v) in extra {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)
}

/// Point every address in an HLS playlist at the proxy.
pub fn rewrite_playlist(text: &str, base: &Url, proxy: impl Fn(&str) -> String) -> String {
    let abs = |u: &str| base.join(u.trim()).map(|x| proxy(x.as_str())).unwrap_or_else(|_| u.to_string());
    let mut out = String::with_capacity(text.len() + 256);
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            out.push('\n');
        } else if let Some(rest) = t.strip_prefix('#') {
            // tags that carry an address: #EXT-X-KEY, #EXT-X-MAP, #EXT-X-MEDIA, #EXT-X-I-FRAME-STREAM-INF ...
            match rest.find("URI=\"") {
                Some(i) => {
                    let start = i + 5;
                    match rest[start..].find('"') {
                        Some(len) => {
                            let uri = &rest[start..start + len];
                            out.push('#');
                            out.push_str(&rest[..start]);
                            out.push_str(&abs(uri));
                            out.push_str(&rest[start + len..]);
                        }
                        None => {
                            out.push_str(t);
                        }
                    }
                }
                None => out.push_str(t),
            }
            out.push('\n');
        } else {
            out.push_str(&abs(t));
            out.push('\n');
        }
    }
    out
}

#[allow(dead_code)]
fn _assert_error_type(_: Error) {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn playlist_addresses_are_rewritten() {
        let base = Url::parse("http://cdn.example/live/u/p/1.m3u8").unwrap();
        let out = rewrite_playlist(
            "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"key.bin\"\n#EXTINF:6,\nseg1.ts\n#EXTINF:6,\nhttp://other.example/a/seg2.ts?x=1\n",
            &base,
            |u| format!("P[{u}]"),
        );
        assert!(out.contains("URI=\"P[http://cdn.example/live/u/p/key.bin]\""), "{out}");
        assert!(out.contains("\nP[http://cdn.example/live/u/p/seg1.ts]\n"), "{out}");
        assert!(out.contains("\nP[http://other.example/a/seg2.ts?x=1]\n"), "{out}");
        assert!(out.starts_with("#EXTM3U\n"));
    }

    /// Fake provider: a playlist with a relative segment, and a segment that supports Range.
    fn provider() -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for c in l.incoming() {
                let Ok(mut s) = c else { break };
                let mut buf = [0u8; 2048];
                let n = s.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let line = req.lines().next().unwrap_or("").to_string();
                let r = if line.contains("/live/1.m3u8") {
                    let b = "#EXTM3U\n#EXTINF:6,\nseg1.ts\n";
                    format!("HTTP/1.1 200 OK\r\nContent-Type: application/x-mpegURL\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{b}", b.len())
                } else if line.contains("/live/seg1.ts") {
                    if req.to_ascii_lowercase().contains("range: bytes=2-5") {
                        "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 2-5/10\r\nContent-Length: 4\r\nConnection: close\r\n\r\n2345".to_string()
                    } else {
                        "HTTP/1.1 200 OK\r\nContent-Type: video/mp2t\r\nContent-Length: 10\r\nConnection: close\r\n\r\n0123456789".to_string()
                    }
                } else {
                    "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string()
                };
                let _ = s.write_all(r.as_bytes());
            }
        });
        format!("http://127.0.0.1:{port}")
    }

    fn fetch(url: &str, range: Option<&str>) -> (u16, String, Vec<(String, String)>) {
        let agent: ureq::Agent = ureq::Agent::config_builder().http_status_as_error(false).build().into();
        let mut req = agent.get(url);
        if let Some(r) = range {
            req = req.header("Range", r);
        }
        let resp = req.call().unwrap();
        let status = resp.status().as_u16();
        let headers = resp.headers().iter().map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string())).collect();
        let mut body = String::new();
        resp.into_body().into_reader().read_to_string(&mut body).unwrap();
        (status, body, headers)
    }

    #[test]
    fn playlist_and_segments_flow_through_and_stay_private() {
        let up = provider();
        let proxy = Proxy::start().unwrap();
        let (st, body, h) = fetch(&proxy.url_for(&format!("{up}/live/1.m3u8")), None);
        assert_eq!(st, 200);
        assert!(h.iter().any(|(k, v)| k == "access-control-allow-origin" && v == "*"));
        // the segment line now points at the proxy, not the provider
        let seg = body.lines().find(|l| l.starts_with("http://127.0.0.1:")).expect(&body).to_string();
        assert!(!body.contains(&up));
        let (st, data, _) = fetch(&seg, None);
        assert_eq!((st, data.as_str()), (200, "0123456789"));
        // seeking works
        let (st, data, h) = fetch(&seg, Some("bytes=2-5"));
        assert_eq!((st, data.as_str()), (206, "2345"));
        assert!(h.iter().any(|(k, _)| k == "content-range"));
        // wrong secret is refused, missing upstream file passes the status through
        let bad = seg.replacen(&proxy.secret.to_string(), "nope", 1);
        assert_eq!(fetch(&bad, None).0, 403);
        assert_eq!(fetch(&proxy.url_for(&format!("{up}/missing")), None).0, 404);
    }
}
