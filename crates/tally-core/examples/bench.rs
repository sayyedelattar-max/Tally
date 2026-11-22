//! Synthetic benchmark: 100k-channel playlist + a 40k-programme guide.
//!
//!     cargo run --release -p tally-core --example bench
//!
//! All data is generated here; no provider content is involved.

use std::io::Cursor;
use std::time::{Duration, Instant};
use tally_core::{Channel, Store, m3u::parse_m3u, store::Direction, xmltv::{parse_xmltv, parse_xmltv_time}};

const CHANNELS: usize = 100_000;
const GUIDE_CHANNELS: usize = 5_000;
const PROGS_PER_CHANNEL: usize = 8;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn pick<'a>(&mut self, xs: &'a [&'a str]) -> &'a str {
        xs[(self.next() % xs.len() as u64) as usize]
    }
}

fn arabic_digits(n: usize) -> String {
    n.to_string().chars().map(|c| char::from_u32('٠' as u32 + c.to_digit(10).unwrap()).unwrap()).collect()
}

fn gen_playlist() -> String {
    let en = ["Harbor", "Meridian", "Atlas", "Oasis", "Lantern", "Falcon", "Delta", "Kite", "Cedar", "Summit", "Ember", "Quartz", "Tidal", "Willow", "Granite", "Compass"];
    let en_sfx = ["News", "Sports", "Cinema", "Docs", "Kids", "Music", "Drama", "Weather", "Classics", "Live"];
    let ar = ["المرفأ", "ميريديان", "أطلس", "الواحة", "فانوس", "الصقر", "دلتا", "الطائرة", "الأرز", "القمة"];
    let ar_sfx = ["نيوز", "سبورت", "سينما", "وثائقية", "أطفال", "ميوزيك", "دراما", "الطقس", "كلاسيك", "مباشر"];
    let groups = ["News", "Sport", "Movies", "Kids", "Music", "Documentary"];
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let mut s = String::with_capacity(CHANNELS * 220);
    s.push_str("#EXTM3U\n");
    for i in 1..=CHANNELS {
        let name = if rng.next() % 10 == 0 {
            format!("{} {} {}", rng.pick(&ar), rng.pick(&ar_sfx), arabic_digits(i % 997))
        } else {
            format!("{} {} {}", rng.pick(&en), rng.pick(&en_sfx), i % 997)
        };
        s.push_str(&format!(
            "#EXTINF:-1 tvg-id=\"ch{i}\" tvg-name=\"{name}\" tvg-logo=\"http://logos.example/{i}.png\" tvg-chno=\"{i}\" group-title=\"{}\",{name}\nhttp://stream.example/live/{i}.ts\n",
            rng.pick(&groups)
        ));
    }
    s
}

fn xmltv_stamp(offset_min: usize) -> String {
    let total = 20 * 60 + offset_min;
    format!("202611{:02}{:02}{:02}00 +0000", 21 + total / 1440, (total % 1440) / 60, total % 60)
}

fn gen_guide() -> String {
    let titles = ["Evening Bulletin", "The Long Table", "Markets Tonight", "Derby Night Live", "Court Side", "Deep Ocean Cities", "Little Lighthouse", "Retro Cassette", "Night Atlas", "Weather & Roads", "الأسواق الليلة", "المائدة الطويلة", "ليلة الديربي", "أطلس الليل"];
    let mut rng = Rng(0xD1B54A32D192ED03);
    let mut s = String::with_capacity(GUIDE_CHANNELS * PROGS_PER_CHANNEL * 220);
    s.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<tv>\n");
    for c in 1..=GUIDE_CHANNELS {
        for p in 0..PROGS_PER_CHANNEL {
            s.push_str(&format!(
                "<programme start=\"{}\" stop=\"{}\" channel=\"ch{c}\"><title>{}</title><desc>Synthetic description for benchmarking.</desc></programme>\n",
                xmltv_stamp(p * 30),
                xmltv_stamp((p + 1) * 30),
                rng.pick(&titles).replace('&', "&amp;")
            ));
        }
    }
    s.push_str("</tv>\n");
    s
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn peak_rss_mb() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| s.lines().find(|l| l.starts_with("VmHWM")).map(str::to_owned))
        .and_then(|l| l.split_whitespace().nth(1).and_then(|n| n.parse::<f64>().ok()))
        .map(|kb| kb / 1024.0)
        .unwrap_or(f64::NAN)
}

fn timed<T>(f: impl FnOnce() -> T) -> (T, Duration) {
    let t = Instant::now();
    let v = f();
    (v, t.elapsed())
}

/// Run `f` `n` times; return (median, p95) in milliseconds.
fn latency(n: usize, mut f: impl FnMut(usize)) -> (f64, f64) {
    let mut v: Vec<f64> = (0..n)
        .map(|i| {
            let t = Instant::now();
            f(i);
            ms(t.elapsed())
        })
        .collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (v[n / 2], v[(n * 95 / 100).min(n - 1)])
}

fn main() {
    let path = std::env::temp_dir().join("tally-bench.db");
    for ext in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{}", path.display(), ext));
    }

    println!("generating synthetic data...");
    let playlist = gen_playlist();
    let guide = gen_guide();
    println!("playlist {:.1} MB, guide {:.1} MB\n", playlist.len() as f64 / 1e6, guide.len() as f64 / 1e6);

    let mut store = Store::open(&path).unwrap();

    let (channels, t_parse) = timed(|| {
        let mut v: Vec<Channel> = Vec::with_capacity(CHANNELS);
        parse_m3u(Cursor::new(playlist.as_bytes()), |c| v.push(c)).unwrap();
        v
    });
    let (_, t_import) = timed(|| store.import_channels(channels).unwrap());

    let (progs, t_gparse) = timed(|| {
        let mut v = Vec::with_capacity(GUIDE_CHANNELS * PROGS_PER_CHANNEL);
        parse_xmltv(Cursor::new(guide.as_bytes()), |p| v.push(p)).unwrap();
        v
    });
    let n_progs = progs.len();
    let (_, t_gimport) = timed(|| store.import_programmes(progs).unwrap());
    drop(store);

    // Cold start: fresh connection to the existing file, then the first screenful.
    let (first_page, t_cold) = timed(|| {
        let s = Store::open(&path).unwrap();
        let page = s.channels_after(None, 50).unwrap();
        (s, page)
    });
    let (store, page) = first_page;
    assert_eq!(page.len(), 50);

    let at = parse_xmltv_time("20261121211000 +0000").unwrap();
    let queries = ["meridian", "sports 12", "harbr", "المرفا", "ep", "قمه سينما", "kite kids 4"];
    println!("{:<34}{:>12}{:>12}", "operation", "median ms", "p95 ms");
    println!("{}", "-".repeat(58));
    for q in queries {
        let (m, p) = latency(40, |_| {
            let _ = store.search(q, 20).unwrap();
        });
        println!("{:<34}{:>12.2}{:>12.2}", format!("search \"{q}\""), m, p);
    }
    let (m, p) = latency(2000, |i| {
        let _ = store.now_next(&format!("ch{}", 1 + i % GUIDE_CHANNELS), at).unwrap();
    });
    println!("{:<34}{:>12.3}{:>12.3}", "now/next lookup", m, p);
    let (m, p) = latency(2000, |i| {
        let _ = store.zap(1 + (i as i64 * 37) % CHANNELS as i64, Direction::Up).unwrap();
    });
    println!("{:<34}{:>12.3}{:>12.3}", "zap up", m, p);
    let (m, p) = latency(2000, |i| {
        let _ = store.channel_by_number(1 + (i as i64 * 53) % CHANNELS as i64).unwrap();
    });
    println!("{:<34}{:>12.3}{:>12.3}", "go to channel number", m, p);
    let mut cursor = Some((50_000, 0));
    let (m, p) = latency(200, |_| {
        let pg = store.channels_after(cursor, 50).unwrap();
        let l = pg.last().unwrap();
        cursor = Some((l.number.unwrap(), l.id));
    });
    println!("{:<34}{:>12.3}{:>12.3}", "scroll page (50 rows, mid-list)", m, p);

    println!("\n{:<34}{:>12}", "import step", "ms");
    println!("{}", "-".repeat(46));
    println!("{:<34}{:>12.0}", format!("parse {CHANNELS} channels"), ms(t_parse));
    println!("{:<34}{:>12.0}", "store channels + build index", ms(t_import));
    println!("{:<34}{:>12.0}", format!("parse {n_progs} programmes"), ms(t_gparse));
    println!("{:<34}{:>12.0}", "store guide + build index", ms(t_gimport));
    println!("\ncold start to first 50 rows: {:.1} ms", ms(t_cold));
    println!("database size: {:.1} MB", std::fs::metadata(&path).unwrap().len() as f64 / 1e6);
    println!("peak memory: {:.0} MB (includes both generated input strings)", peak_rss_mb());
}
