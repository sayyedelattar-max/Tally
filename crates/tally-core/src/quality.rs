//! Providers list one channel many times, once per quality ("BEIN SPORT 1 HQ 4M", "… H265 FHD 4M").
//! This groups those lines back into one channel with a list of qualities. It only reads names:
//! nothing is changed in the database, so the provider's own list is always still there.

use crate::store::ChannelRow;

#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    pub channel: ChannelRow,
    /// What was tacked onto the name, e.g. `H265 FHD 4M` (`Standard` when nothing was).
    pub label: String,
    /// Higher is better. Only meant for ordering.
    pub score: f64,
    pub hevc: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Merged {
    pub key: String,
    pub name: String,
    pub variants: Vec<Variant>,
}

fn bitrate(tok: &str) -> Option<f64> {
    // 512K, 2M, 4/8M, 8M
    let (num, unit) = tok.split_at(tok.len().checked_sub(1)?);
    let unit = match unit {
        "M" => 1.0,
        "K" => 0.001,
        _ => return None,
    };
    if num.is_empty() || !num.chars().all(|c| c.is_ascii_digit() || c == '/') || num.starts_with('/') || num.ends_with('/') {
        return None;
    }
    num.split('/').filter_map(|p| p.parse::<f64>().ok()).fold(None, |m: Option<f64>, v| Some(m.map_or(v, |x| x.max(v)))).map(|v| v * unit)
}

fn tier(tok: &str) -> Option<f64> {
    Some(match tok {
        "MINI" | "NANO" | "LOW" | "LITE" => 0.0,
        "SD" => 1.0,
        "HQ" | "PURE" => 2.0,
        "HD" | "720P" | "1080I" => 3.0,
        "FHD" | "1080P" | "POWER" | "MAX" => 4.0,
        "UHD" | "4K" | "2160P" | "ULTRA" => 5.0,
        "8K" => 6.0,
        _ => return None,
    })
}

fn is_codec(tok: &str) -> bool {
    matches!(tok, "H265" | "HEVC" | "H.265" | "H264" | "H.264" | "AVC")
}

fn is_noise(tok: &str) -> bool {
    matches!(tok, "50FPS" | "60FPS" | "25FPS" | "30FPS" | "|" | "-")
}

fn is_quality(tok: &str) -> bool {
    tier(tok).is_some() || is_codec(tok) || is_noise(tok) || bitrate(tok).is_some()
}

/// Split a channel name into its base and its quality words.
pub fn split(name: &str) -> (String, String, f64, bool) {
    let toks: Vec<&str> = name.split_whitespace().collect();
    let mut end = toks.len();
    while end > 1 && is_quality(&toks[end - 1].to_uppercase()) {
        end -= 1;
    }
    let base = toks[..end].join(" ");
    let rest = &toks[end..];
    let (mut score, mut hevc) = (0.0, false);
    for t in rest {
        let u = t.to_uppercase();
        if let Some(v) = tier(&u) {
            score += v * 10.0;
        }
        if let Some(b) = bitrate(&u) {
            score += b.min(20.0);
        }
        if matches!(u.as_str(), "H265" | "HEVC" | "H.265") {
            hevc = true;
        }
    }
    let label = rest.iter().filter(|t| !is_noise(&t.to_uppercase())).cloned().collect::<Vec<_>>().join(" ");
    (base, if label.is_empty() { "Standard".into() } else { label }, score, hevc)
}

/// Group channels of the same category that differ only by quality, keeping the provider's order
/// (a group appears where its first member did). Lines that have no twin come back as they were.
pub fn merge(rows: Vec<ChannelRow>) -> Vec<Merged> {
    let mut out: Vec<Merged> = Vec::new();
    let mut index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for ch in rows {
        let (base, label, score, hevc) = split(&ch.name);
        let key = format!("{}|{}", ch.group, base.to_uppercase());
        let v = Variant { channel: ch, label, score, hevc };
        match index.get(&key) {
            Some(&i) => out[i].variants.push(v),
            None => {
                index.insert(key.clone(), out.len());
                out.push(Merged { key, name: base, variants: vec![v] });
            }
        }
    }
    for m in &mut out {
        if m.variants.len() == 1 {
            m.name = m.variants[0].channel.name.clone(); // nothing to merge: keep the real name
        } else {
            m.variants.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(id: i64, name: &str, group: &str) -> ChannelRow {
        ChannelRow { id, tvg_id: String::new(), name: name.into(), group: group.into(), logo: String::new(), url: String::new(), number: None, fav: false, hidden: false }
    }

    #[test]
    fn provider_lines_become_one_channel() {
        let names = [
            "BEIN SPORT 1 MINI 512K", "BEIN SPORT 1 LOW 1M", "BEIN SPORT 1 SD 2M", "BEIN SPORT 1 NANO 2M", "BEIN SPORT 1 HQ 4M",
            "BEIN SPORT 1 PURE 4/8M", "BEIN SPORT 1 H265 HQ 1M", "BEIN SPORT 1 H265 HD 2M", "BEIN SPORT 1 H265 FHD 4M", "BEIN SPORT 1 H265 POWER 8M",
        ];
        let rows: Vec<_> = names.iter().enumerate().map(|(i, n)| ch(i as i64, n, "EGYPT")).collect();
        let m = merge(rows);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].name, "BEIN SPORT 1");
        assert_eq!(m[0].variants.len(), 10);
        // best first, and the labels keep what the provider wrote
        let labels: Vec<_> = m[0].variants.iter().map(|v| v.label.as_str()).collect();
        assert!(labels[0].starts_with("H265 POWER") || labels[0].starts_with("H265 FHD"));
        assert_eq!(labels.last().unwrap(), &"MINI 512K");
        assert!(m[0].variants.iter().filter(|v| v.hevc).count() == 4);
    }

    #[test]
    fn different_numbers_categories_and_lone_channels_stay_apart() {
        let m = merge(vec![
            ch(1, "BEIN SPORT 1 HD", "A"), ch(2, "BEIN SPORT 2 HD", "A"), ch(3, "BEIN SPORT 1 HD", "B"),
            ch(4, "AFC HD", "A"), ch(5, "MBC 4", "A"), ch(6, "MBC 4 HD", "A"),
        ]);
        assert_eq!(m.len(), 5);
        assert!(m.iter().any(|x| x.name == "AFC HD"));
        let mbc = m.iter().find(|x| x.name == "MBC 4").unwrap();
        assert_eq!(mbc.variants.len(), 2);
        assert!(mbc.variants.iter().any(|v| v.label == "Standard"));
    }

    #[test]
    fn a_name_that_is_all_quality_words_is_left_alone() {
        let (base, _, _, _) = split("HD");
        assert_eq!(base, "HD");
    }
}
