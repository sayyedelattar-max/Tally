//! Streaming XMLTV parser.
//!
//! Guide files for large providers are tens or hundreds of megabytes. This reads
//! events one at a time and hands each `<programme>` to a callback, so memory
//! stays flat no matter how big the file is.

use crate::error::{Error, Result};
use crate::model::Programme;
use quick_xml::{Reader, events::Event};
use std::io::BufRead;

#[derive(PartialEq, Clone, Copy)]
enum Field {
    None,
    Title,
    Desc,
}

pub fn parse_xmltv<R: BufRead>(reader: R, mut sink: impl FnMut(Programme)) -> Result<usize> {
    let mut rd = Reader::from_reader(reader);
    let mut buf = Vec::new();
    let mut count = 0usize;

    let mut cur: Option<Programme> = None;
    let mut field = Field::None;
    let (mut title_done, mut desc_done) = (false, false);

    loop {
        let ev = rd
            .read_event_into(&mut buf)
            .map_err(|e| Error::Xml(format!("near byte {}: {e}", rd.buffer_position())))?;
        match ev {
            Event::Eof => break,
            Event::Start(e) => match e.name().as_ref() {
                "programme" => {
                    let (mut start, mut stop, mut channel) = (None, None, String::new());
                    for a in e.attributes().flatten() {
                        let v = &a.value;
                        match a.key.as_ref() {
                            "start" => start = parse_xmltv_time(&v),
                            "stop" => stop = parse_xmltv_time(&v),
                            "channel" => channel = v.to_string(),
                            _ => {}
                        }
                    }
                    cur = match (start, stop) {
                        (Some(start), Some(stop)) => Some(Programme {
                            channel,
                            start,
                            stop,
                            title: String::new(),
                            desc: String::new(),
                        }),
                        _ => None, // malformed entry: skip it
                    };
                    (title_done, desc_done, field) = (false, false, Field::None);
                }
                "title" if cur.is_some() && !title_done => field = Field::Title,
                "desc" if cur.is_some() && !desc_done => field = Field::Desc,
                _ => {}
            },
            Event::Text(t) => push_text(&mut cur, field, &t),
            Event::GeneralRef(r) => {
                // &amp; &lt; &#1575; etc. arrive as their own events
                let resolved = match r.resolve_char_ref() {
                    Ok(Some(c)) => Some(c.to_string()),
                    _ => match r.as_ref() {
                        "amp" => Some("&".into()),
                        "lt" => Some("<".into()),
                        "gt" => Some(">".into()),
                        "quot" => Some("\"".into()),
                        "apos" => Some("'".into()),
                        _ => None,
                    },
                };
                if let Some(s) = resolved {
                    push_text(&mut cur, field, &s);
                }
            }
            Event::End(e) => match e.name().as_ref() {
                "title" => {
                    if field == Field::Title {
                        title_done = true;
                    }
                    field = Field::None;
                }
                "desc" => {
                    if field == Field::Desc {
                        desc_done = true;
                    }
                    field = Field::None;
                }
                "programme" => {
                    if let Some(p) = cur.take() {
                        sink(p);
                        count += 1;
                    }
                }
                _ => {}
            },
            _ => {}
        }
        buf.clear();
    }
    Ok(count)
}

fn push_text(cur: &mut Option<Programme>, field: Field, s: &str) {
    if let Some(p) = cur.as_mut() {
        match field {
            Field::Title => p.title.push_str(s),
            Field::Desc => p.desc.push_str(s),
            Field::None => {}
        }
    }
}

/// `20261121201000 +0200` -> Unix seconds (UTC). Seconds and zone are optional.
pub fn parse_xmltv_time(s: &str) -> Option<i64> {
    let s = s.trim();
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    let (d, rest) = s.split_at(end);
    if d.len() < 8 || d.len() > 14 {
        return None;
    }
    let num = |a: usize, b: usize| -> Option<i64> { d.get(a..b).and_then(|x| x.parse().ok()) };
    let (y, mo, da) = (num(0, 4)?, num(4, 6)?, num(6, 8)?);
    let (h, mi, se) = (num(8, 10).unwrap_or(0), num(10, 12).unwrap_or(0), num(12, 14).unwrap_or(0));
    let mut ts = days_from_civil(y, mo, da) * 86_400 + h * 3600 + mi * 60 + se;

    let rest = rest.trim();
    if let Some(sign @ ('+' | '-')) = rest.chars().next() {
        let tz = rest[1..].replace(':', "");
        let hh: i64 = tz.get(0..2)?.parse().ok()?;
        let mm: i64 = tz.get(2..4).and_then(|x| x.parse().ok()).unwrap_or(0);
        let off = hh * 3600 + mm * 60;
        ts -= if sign == '+' { off } else { -off };
    }
    Some(ts)
}

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12; // March = 0
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn time_parsing() {
        assert_eq!(parse_xmltv_time("19700101000000 +0000"), Some(0));
        assert_eq!(parse_xmltv_time("20000301000000 +0000"), Some(951_868_800));
        // same instant written in +02:00
        assert_eq!(parse_xmltv_time("20000301020000 +0200"), Some(951_868_800));
        // zone and seconds are optional
        assert_eq!(parse_xmltv_time("200003010000"), Some(951_868_800));
        assert_eq!(parse_xmltv_time("garbage"), None);
    }

    #[test]
    fn streams_programmes_with_entities_and_arabic() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<tv>
  <channel id="hn.example"><display-name>Harbor News 24</display-name></channel>
  <programme start="20261121200000 +0200" stop="20261121204500 +0200" channel="hn.example">
    <title lang="en">Markets &amp; Money</title>
    <title lang="ar">الأسواق</title>
    <desc>Daily wrap &#8211; live</desc>
  </programme>
  <programme start="oops" stop="20261121210000 +0200" channel="hn.example"><title>Skipped</title></programme>
  <programme start="20261121204500 +0200" stop="20261121214500 +0200" channel="hn.example">
    <title>المائدة الطويلة</title>
  </programme>
</tv>"#;
        let mut v = Vec::new();
        let n = parse_xmltv(Cursor::new(xml), |p| v.push(p)).unwrap();
        assert_eq!(n, 2);
        assert_eq!(v[0].title, "Markets & Money"); // first <title> only, entity decoded
        assert_eq!(v[0].desc, "Daily wrap \u{2013} live");
        assert_eq!(v[0].stop - v[0].start, 45 * 60);
        assert_eq!(v[1].title, "المائدة الطويلة");
    }
}
