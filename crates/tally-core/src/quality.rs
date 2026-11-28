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
    let last = tok.chars().next_back()?;
    let num = &tok[..tok.len() - last.len_utf8()];
    let unit = match last {
        'M' => 1.0,
        'K' => 0.001,
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

/// Tokens of a base name, upper-cased, with quality letters stuck to a number dropped:
/// `Bein Movies HD1` and `BEIN MOVIES 1` both give `BEIN MOVIES 1`; `STC TV 1HD` gives `STC TV 1`.
fn canon(base: &str) -> Vec<String> {
    base.split_whitespace()
        .map(|t| {
            let u = t.to_uppercase();
            let digits: String = u.chars().filter(|c| c.is_ascii_digit()).collect();
            let letters: String = u.chars().filter(|c| !c.is_ascii_digit()).collect();
            if !digits.is_empty() && !letters.is_empty() && (tier(&letters).is_some() || is_codec(&letters)) {
                digits
            } else {
                u
            }
        })
        .collect()
}

/// `BEIN SPORT 3` → (`BEIN SPORT`, 3): the channel's family and its number, when it has one.
fn series_of(base: &str) -> Option<(String, u32)> {
    let toks = canon(base);
    let i = toks.iter().rposition(|t| !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()))?;
    let n: u32 = toks[i].parse().ok()?;
    let rest: Vec<&str> = toks.iter().enumerate().filter(|(j, t)| *j != i && !is_quality(t)).map(|(_, t)| t.as_str()).collect();
    if rest.is_empty() {
        return None;
    }
    Some((rest.join(" "), n))
}

/// Put numbered channels of one family next to each other in numeric order (BEIN SPORT 1, 2, 3 …),
/// the family sitting where its first member was. Everything else keeps its place.
fn cluster(list: Vec<Merged>) -> Vec<Merged> {
    let info: Vec<Option<(String, u32)>> = list.iter().map(|m| series_of(&split(&m.variants[0].channel.name).0)).collect();
    let mut count: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for (s, _) in info.iter().flatten() {
        *count.entry(s.as_str()).or_default() += 1;
    }
    let mut taken = vec![false; list.len()];
    let mut order: Vec<usize> = Vec::with_capacity(list.len());
    for i in 0..list.len() {
        if taken[i] {
            continue;
        }
        match &info[i] {
            Some((s, _)) if count[s.as_str()] >= 2 => {
                let mut members: Vec<usize> = (i..list.len()).filter(|&j| !taken[j] && info[j].as_ref().is_some_and(|x| x.0 == *s)).collect();
                members.sort_by_key(|&j| (info[j].as_ref().map(|x| x.1).unwrap_or(0), j));
                for j in members {
                    taken[j] = true;
                    order.push(j);
                }
            }
            _ => {
                taken[i] = true;
                order.push(i);
            }
        }
    }
    let mut slots: Vec<Option<Merged>> = list.into_iter().map(Some).collect();
    order.into_iter().filter_map(|i| slots[i].take()).collect()
}

/// The category name without its trailing quality words, e.g. `BEIN SPORT H265 FHD 4M` → `BEIN SPORT`.
/// Providers often make one category per quality; categories with the same family belong together.
pub fn family(group: &str) -> String {
    let (base, _, _, _) = split(group.trim());
    base
}

/// Quality words found on the category name (empty when there are none).
fn group_tier(group: &str) -> Option<(String, f64, bool)> {
    let (base, label, score, hevc) = split(group.trim());
    if base == group.trim() || label == "Standard" {
        None
    } else {
        Some((label, score, hevc))
    }
}

/// Group channels of the same category that differ only by quality, keeping the provider's order
/// (a group appears where its first member did). Lines that have no twin come back as they were.
pub fn merge(rows: Vec<ChannelRow>) -> Vec<Merged> {
    merge_with(rows, true, true)
}

/// `across`: fold categories that are one quality of the same family. `lines`: merge repeated
/// qualities of a channel at all (off = every line stays its own entry).
pub fn merge_with(rows: Vec<ChannelRow>, across: bool, lines: bool) -> Vec<Merged> {
    if !lines {
        return rows
            .into_iter()
            .map(|ch| Merged { key: format!("id:{}", ch.id), name: ch.name.clone(), variants: vec![Variant { label: "Standard".into(), score: 0.0, hevc: false, channel: ch }] })
            .collect();
    }
    let mut out: Vec<Merged> = Vec::new();
    let mut index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for ch in rows {
        let (base, mut label, mut score, mut hevc) = split(&ch.name);
        // the quality may live on the category ("BEIN SPORT FHD 4M") instead of on the channel
        if label == "Standard" {
            if let Some((l, s, h)) = group_tier(&ch.group) {
                (label, score, hevc) = (l, s, h);
            }
        }
        let cat = if across { family(&ch.group) } else { ch.group.clone() };
        let key = format!("{}|{}", cat.to_uppercase(), canon(&base).join(" "));
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
    cluster(out)
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
    fn one_category_per_quality_still_becomes_one_channel() {
        let m = merge(vec![
            ch(1, "BEIN SPORT 1 HQ 4M", "BEIN SPORT HQ 4M"), ch(2, "BEIN SPORT 2 HQ 4M", "BEIN SPORT HQ 4M"),
            ch(3, "BEIN SPORT 1 LOW 1M", "BEIN SPORT LOW 1M"), ch(4, "BEIN SPORT 2 LOW 1M", "BEIN SPORT LOW 1M"),
            ch(5, "BEIN SPORT 1", "BEIN SPORT H265 FHD 4M"),
        ]);
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].name, "BEIN SPORT 1");
        assert_eq!(m[0].variants.len(), 3);
        assert_eq!(m[0].variants[0].label, "H265 FHD 4M"); // quality taken from the category
        assert_eq!(family("BEIN SPORT PURE 4/8M"), "BEIN SPORT");
        assert_eq!(family("ARABIC SPORTS"), "ARABIC SPORTS");
    }

    #[test]
    fn numbered_channels_of_a_family_sit_together_in_order() {
        let m = merge(vec![
            ch(1, "STATUS", "A"), ch(2, "BEIN SPORT 3 HQ 4M", "BEIN SPORT HQ 4M"), ch(3, "NOVA NEWS", "A"), ch(4, "BEIN SPORT 1 HQ 4M", "BEIN SPORT HQ 4M"),
            ch(5, "ON SPORT", "C"), ch(6, "BEIN SPORT 2 HQ 4M", "BEIN SPORT HQ 4M"), ch(7, "Bein Movies HD1", "BEIN MOVIES HD"), ch(8, "beIN MOVIES 1 4K", "BEIN MOVIES 4K H265"),
            ch(9, "BEIN SPORT 1 LOW 1M", "BEIN SPORT LOW 1M"),
        ]);
        let names: Vec<_> = m.iter().map(|x| x.name.as_str()).collect();
        assert_eq!(&names[..5], ["STATUS", "BEIN SPORT 1", "BEIN SPORT 2 HQ 4M", "BEIN SPORT 3 HQ 4M", "NOVA NEWS"]);
        assert_eq!(m[1].variants.len(), 2); // 1 HQ + 1 LOW
        let movies: Vec<_> = m.iter().filter(|x| x.name.to_uppercase().contains("MOVIES")).collect();
        assert_eq!(movies.len(), 1); // HD1 and "1 4K" are the same channel
        assert_eq!(movies[0].variants.len(), 2);
    }

    #[test]
    fn arabic_and_other_non_ascii_names_do_not_panic() {
        let m = merge(vec![ch(1, "قناة الجزيرة HD", "A"), ch(2, "الجزيرة مباشر", "A"), ch(3, "ى", "A"), ch(4, "قناة ى 4M", "A"), ch(5, "😀 HD", "A")]);
        assert!(!m.is_empty());
        assert_eq!(split("قناة الجزيرة HD").0, "قناة الجزيرة");
    }

    #[test]
    fn a_name_that_is_all_quality_words_is_left_alone() {
        let (base, _, _, _) = split("HD");
        assert_eq!(base, "HD");
    }
}
