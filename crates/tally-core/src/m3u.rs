//! Streaming M3U / M3U8 parser.
//!
//! Reads line by line and hands each channel to a callback, so a 100k-entry
//! playlist never has to exist in memory as a whole.

use crate::model::Channel;
use std::io::BufRead;

/// Parse a playlist. Returns the number of channels delivered to `sink`.
pub fn parse_m3u<R: BufRead>(reader: R, mut sink: impl FnMut(Channel)) -> std::io::Result<usize> {
    let mut pending: Option<Channel> = None;
    let mut group_line: Option<String> = None;
    let mut count = 0usize;

    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        let line = if i == 0 { line.trim_start_matches('\u{feff}') } else { line.as_str() };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("#EXTINF:") {
            pending = Some(parse_extinf(rest));
        } else if let Some(g) = line.strip_prefix("#EXTGRP:") {
            group_line = Some(g.trim().to_string());
        } else if line.starts_with('#') {
            // #EXTM3U, #EXTVLCOPT, comments: nothing we need
        } else if let Some(mut ch) = pending.take() {
            ch.url = line.to_string();
            if ch.group.is_empty() {
                if let Some(g) = group_line.take() {
                    ch.group = g;
                }
            }
            if ch.name.is_empty() {
                ch.name = ch.url.clone();
            }
            sink(ch);
            count += 1;
            group_line = None;
        } else {
            // a bare URL with no #EXTINF: keep it, named after its URL
            sink(Channel { name: line.to_string(), url: line.to_string(), ..Default::default() });
            count += 1;
        }
    }
    Ok(count)
}

/// `rest` is everything after `#EXTINF:`, e.g.
/// `-1 tvg-id="hn" group-title="News, World",Harbor News 24`
fn parse_extinf(rest: &str) -> Channel {
    // The first comma outside quotes separates attributes from the display name.
    let mut in_quotes = false;
    let mut split = rest.len();
    for (i, c) in rest.char_indices() {
        match c {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                split = i;
                break;
            }
            _ => {}
        }
    }
    let (head, name) = rest.split_at(split);
    let name = name.strip_prefix(',').unwrap_or("").trim();

    let mut ch = Channel { name: name.to_string(), ..Default::default() };
    let mut tvg_name = String::new();

    let bytes = head.as_bytes();
    let mut i = 0;
    // skip the duration token
    while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let key_start = i;
        while i < bytes.len() && bytes[i] != b'=' && !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            continue;
        }
        let key = &head[key_start..i];
        i += 1; // '='
        let value;
        if i < bytes.len() && bytes[i] == b'"' {
            i += 1;
            let v_start = i;
            while i < bytes.len() && bytes[i] != b'"' {
                i += 1;
            }
            value = &head[v_start..i];
            i += 1; // closing quote
        } else {
            let v_start = i;
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            value = &head[v_start..i];
        }
        match key.to_ascii_lowercase().as_str() {
            "tvg-id" => ch.tvg_id = value.to_string(),
            "tvg-name" => tvg_name = value.to_string(),
            "tvg-logo" => ch.logo = value.to_string(),
            "group-title" => ch.group = value.to_string(),
            "tvg-chno" | "channel-number" => ch.number = value.trim().parse().ok(),
            _ => {}
        }
    }
    if ch.name.is_empty() {
        ch.name = tvg_name;
    }
    ch
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn parse(s: &str) -> Vec<Channel> {
        let mut v = Vec::new();
        parse_m3u(Cursor::new(s), |c| v.push(c)).unwrap();
        v
    }

    #[test]
    fn parses_attributes_and_name() {
        let v = parse(
            "\u{feff}#EXTM3U\n\
             #EXTINF:-1 tvg-id=\"hn.example\" tvg-logo=\"http://x/l.png\" tvg-chno=\"101\" group-title=\"News, World\",Harbor News 24\n\
             http://s/hn.ts\n",
        );
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].tvg_id, "hn.example");
        assert_eq!(v[0].group, "News, World"); // comma inside quotes survives
        assert_eq!(v[0].name, "Harbor News 24");
        assert_eq!(v[0].number, Some(101));
        assert_eq!(v[0].url, "http://s/hn.ts");
    }

    #[test]
    fn arabic_names_and_extgrp() {
        let v = parse("#EXTM3U\n#EXTINF:-1,المرفأ نيوز ٢٤\n#EXTGRP:أخبار\nhttp://s/1\n");
        assert_eq!(v[0].name, "المرفأ نيوز ٢٤");
        assert_eq!(v[0].group, "أخبار");
    }

    #[test]
    fn falls_back_to_tvg_name_and_handles_bare_urls() {
        let v = parse("#EXTINF:-1 tvg-name=\"Delta Music\",\nhttp://s/2\nhttp://s/bare\n");
        assert_eq!(v[0].name, "Delta Music");
        assert_eq!(v[1].url, "http://s/bare");
        assert_eq!(v.len(), 2);
    }
}
