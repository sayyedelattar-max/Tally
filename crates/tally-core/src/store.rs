//! SQLite store: channels, guide, favorites and search.
//!
//! Design notes
//! * One file, WAL mode, so the UI can read while an import runs.
//! * Search uses FTS5 with the `trigram` tokenizer over *normalised* text. That
//!   gives substring matching for Latin and Arabic alike without a language model.
//! * Typos: when an exact search finds little, candidates are pulled with an OR of
//!   the query's trigrams and re-ranked by approximate-substring edit distance.
//! * Paging is keyset-based (`channels_after`) so scrolling 100k rows never gets slower.

use crate::error::Result;
use crate::model::{Channel, Programme};
use crate::norm::normalize;
use rusqlite::{Connection, params};
use std::path::Path;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS channels(
  id INTEGER PRIMARY KEY,
  key TEXT NOT NULL UNIQUE,
  tvg_id TEXT NOT NULL DEFAULT '',
  name TEXT NOT NULL,
  name_norm TEXT NOT NULL,
  grp TEXT NOT NULL DEFAULT '',
  logo TEXT NOT NULL DEFAULT '',
  url TEXT NOT NULL,
  number INTEGER,
  fav INTEGER NOT NULL DEFAULT 0,
  gen INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS channels_number ON channels(number, id);
CREATE INDEX IF NOT EXISTS channels_tvg ON channels(tvg_id);

CREATE TABLE IF NOT EXISTS programmes(
  id INTEGER PRIMARY KEY,
  channel TEXT NOT NULL,
  start INTEGER NOT NULL,
  stop INTEGER NOT NULL,
  title TEXT NOT NULL,
  title_norm TEXT NOT NULL,
  descr TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS programmes_ch_start ON programmes(channel, start);

CREATE VIRTUAL TABLE IF NOT EXISTS channel_fts
  USING fts5(name_norm, content='channels', content_rowid='id', tokenize='trigram');
CREATE VIRTUAL TABLE IF NOT EXISTS programme_fts
  USING fts5(title_norm, content='programmes', content_rowid='id', tokenize='trigram');
";

const CH_COLS: &str = "id, tvg_id, name, grp, logo, url, number, fav";
const PR_COLS: &str = "id, channel, start, stop, title, descr";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelRow {
    pub id: i64,
    pub tvg_id: String,
    pub name: String,
    pub group: String,
    pub logo: String,
    pub url: String,
    pub number: Option<i64>,
    pub fav: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgrammeRow {
    pub id: i64,
    pub channel: String,
    pub start: i64,
    pub stop: i64,
    pub title: String,
    pub desc: String,
}

#[derive(Debug, Default)]
pub struct SearchResults {
    pub channels: Vec<ChannelRow>,
    pub programmes: Vec<ProgrammeRow>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

pub struct Store {
    conn: Connection,
}

fn channel_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<ChannelRow> {
    Ok(ChannelRow {
        id: r.get(0)?,
        tvg_id: r.get(1)?,
        name: r.get(2)?,
        group: r.get(3)?,
        logo: r.get(4)?,
        url: r.get(5)?,
        number: r.get(6)?,
        fav: r.get::<_, i64>(7)? != 0,
    })
}

fn programme_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<ProgrammeRow> {
    Ok(ProgrammeRow {
        id: r.get(0)?,
        channel: r.get(1)?,
        start: r.get(2)?,
        stop: r.get(3)?,
        title: r.get(4)?,
        desc: r.get(5)?,
    })
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        Self::init(Connection::open(path)?)
    }

    pub fn open_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA temp_store=MEMORY;
             PRAGMA cache_size=-65536; PRAGMA mmap_size=268435456;",
        )?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    // ---------------------------------------------------------------- import

    /// Upsert a playlist. Favorites survive re-imports; channels that disappeared
    /// from the playlist are removed. Channels without a number get their position.
    pub fn import_channels<I: IntoIterator<Item = Channel>>(&mut self, items: I) -> Result<usize> {
        let generation: i64 = self
            .conn
            .query_row("SELECT COALESCE(MAX(gen), 0) + 1 FROM channels", [], |r| r.get(0))?;
        let tx = self.conn.transaction()?;
        let mut n = 0usize;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO channels(key, tvg_id, name, name_norm, grp, logo, url, number, gen)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(key) DO UPDATE SET
                   tvg_id=excluded.tvg_id, name=excluded.name, name_norm=excluded.name_norm,
                   grp=excluded.grp, logo=excluded.logo, url=excluded.url,
                   number=excluded.number, gen=excluded.gen",
            )?;
            for ch in items {
                n += 1;
                let key = if ch.tvg_id.is_empty() {
                    ch.url.clone()
                } else {
                    format!("{}|{}", ch.tvg_id, ch.name)
                };
                let number = ch.number.map(i64::from).unwrap_or(n as i64);
                stmt.execute(params![
                    key,
                    ch.tvg_id,
                    ch.name,
                    normalize(&ch.name),
                    ch.group,
                    ch.logo,
                    ch.url,
                    number,
                    generation
                ])?;
            }
        }
        tx.execute("DELETE FROM channels WHERE gen <> ?1", [generation])?;
        tx.execute("INSERT INTO channel_fts(channel_fts) VALUES('rebuild')", [])?;
        tx.commit()?;
        Ok(n)
    }

    /// Replace the whole guide. XMLTV files are full snapshots, so this is a swap.
    pub fn import_programmes<I: IntoIterator<Item = Programme>>(&mut self, items: I) -> Result<usize> {
        self.try_import_programmes(items.into_iter().map(Ok))
    }

    /// Like [`Store::import_programmes`], but the source can fail part-way (a dropped
    /// download, say). The first `Err` aborts the import and rolls back, so the previous
    /// guide stays intact instead of being replaced by a half-finished one.
    pub fn try_import_programmes<I: IntoIterator<Item = Result<Programme>>>(&mut self, items: I) -> Result<usize> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM programmes", [])?;
        let mut n = 0usize;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO programmes(channel, start, stop, title, title_norm, descr)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;
            for p in items {
                let p = p?; // early return drops `tx` without committing: rollback
                stmt.execute(params![p.channel, p.start, p.stop, p.title, normalize(&p.title), p.desc])?;
                n += 1;
            }
        }
        tx.execute("INSERT INTO programme_fts(programme_fts) VALUES('rebuild')", [])?;
        tx.commit()?;
        Ok(n)
    }

    // ---------------------------------------------------------------- browse

    pub fn channel_count(&self) -> Result<i64> {
        Ok(self.conn.query_row("SELECT COUNT(*) FROM channels", [], |r| r.get(0))?)
    }

    /// Keyset paging in channel-number order. Pass `None` for the first page, then
    /// the `(number, id)` of the last row you received.
    pub fn channels_after(&self, cursor: Option<(i64, i64)>, limit: usize) -> Result<Vec<ChannelRow>> {
        let (n, id) = cursor.unwrap_or((i64::MIN, i64::MIN));
        let sql = format!(
            "SELECT {CH_COLS} FROM channels WHERE (number, id) > (?1, ?2)
             ORDER BY number, id LIMIT ?3"
        );
        let mut st = self.conn.prepare_cached(&sql)?;
        let rows = st.query_map(params![n, id, limit as i64], channel_row)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn channel_by_number(&self, number: i64) -> Result<Option<ChannelRow>> {
        let sql = format!("SELECT {CH_COLS} FROM channels WHERE number = ?1 ORDER BY id LIMIT 1");
        let mut st = self.conn.prepare_cached(&sql)?;
        let mut rows = st.query_map([number], channel_row)?;
        Ok(rows.next().transpose()?)
    }

    /// Next or previous channel by number, wrapping at the ends (the up/down zap keys).
    pub fn zap(&self, current_number: i64, dir: Direction) -> Result<Option<ChannelRow>> {
        let (cmp, order, wrap) = match dir {
            Direction::Up => (">", "number, id", "number, id"),
            Direction::Down => ("<", "number DESC, id DESC", "number DESC, id DESC"),
        };
        let sql = format!(
            "SELECT {CH_COLS} FROM channels WHERE number {cmp} ?1 ORDER BY {order} LIMIT 1"
        );
        let mut st = self.conn.prepare_cached(&sql)?;
        if let Some(r) = st.query_map([current_number], channel_row)?.next().transpose()? {
            return Ok(Some(r));
        }
        let sql = format!("SELECT {CH_COLS} FROM channels WHERE number IS NOT NULL ORDER BY {wrap} LIMIT 1");
        let mut st = self.conn.prepare_cached(&sql)?;
        Ok(st.query_map([], channel_row)?.next().transpose()?)
    }

    pub fn set_favorite(&self, id: i64, fav: bool) -> Result<()> {
        self.conn
            .execute("UPDATE channels SET fav = ?2 WHERE id = ?1", params![id, fav as i64])?;
        Ok(())
    }

    pub fn favorites(&self) -> Result<Vec<ChannelRow>> {
        let sql = format!("SELECT {CH_COLS} FROM channels WHERE fav = 1 ORDER BY number, id");
        let mut st = self.conn.prepare_cached(&sql)?;
        let rows = st.query_map([], channel_row)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    // ----------------------------------------------------------------- guide

    /// What is on at `at`, and what follows it.
    pub fn now_next(&self, tvg_id: &str, at: i64) -> Result<(Option<ProgrammeRow>, Option<ProgrammeRow>)> {
        let sql = format!(
            "SELECT {PR_COLS} FROM programmes WHERE channel = ?1 AND stop > ?2 ORDER BY start LIMIT 2"
        );
        let mut st = self.conn.prepare_cached(&sql)?;
        let rows: Vec<ProgrammeRow> = st
            .query_map(params![tvg_id, at], programme_row)?
            .collect::<std::result::Result<_, _>>()?;
        let mut it = rows.into_iter();
        let first = it.next();
        let second = it.next();
        Ok(match first {
            Some(f) if f.start <= at => (Some(f), second),
            Some(f) => (None, Some(f)),
            None => (None, None),
        })
    }

    /// Everything overlapping `[from, to)` for one channel: one row of the guide grid.
    pub fn guide_window(&self, tvg_id: &str, from: i64, to: i64) -> Result<Vec<ProgrammeRow>> {
        let sql = format!(
            "SELECT {PR_COLS} FROM programmes WHERE channel = ?1 AND stop > ?2 AND start < ?3 ORDER BY start"
        );
        let mut st = self.conn.prepare_cached(&sql)?;
        let rows = st.query_map(params![tvg_id, from, to], programme_row)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    // ---------------------------------------------------------------- search

    pub fn search(&self, query: &str, limit: usize) -> Result<SearchResults> {
        let q = normalize(query);
        if q.is_empty() {
            return Ok(SearchResults::default());
        }
        Ok(SearchResults {
            channels: self.search_channels(&q, limit)?,
            programmes: self.search_programmes(&q, limit)?,
        })
    }

    fn search_channels(&self, q: &str, limit: usize) -> Result<Vec<ChannelRow>> {
        let qlen = q.chars().count();
        let mut out: Vec<ChannelRow> = Vec::new();

        if qlen < 3 {
            let sql = format!(
                "SELECT {CH_COLS} FROM channels WHERE instr(name_norm, ?1) > 0 ORDER BY number, id LIMIT ?2"
            );
            let mut st = self.conn.prepare_cached(&sql)?;
            let rows = st.query_map(params![q, limit as i64], channel_row)?;
            return Ok(rows.collect::<std::result::Result<_, _>>()?);
        }

        let sql = format!(
            "SELECT c.id, c.tvg_id, c.name, c.grp, c.logo, c.url, c.number, c.fav
             FROM channel_fts f JOIN channels c ON c.id = f.rowid
             WHERE channel_fts MATCH ?1 ORDER BY rank LIMIT ?2"
        );
        {
            let mut st = self.conn.prepare_cached(&sql)?;
            let rows = st.query_map(params![phrase(q), limit as i64], channel_row)?;
            for r in rows {
                out.push(r?);
            }
        }

        if out.len() < limit.min(5) && qlen >= 4 {
            let cands = self.fuzzy_candidates("channel_fts", "channels", "name_norm", q, 300)?;
            let ids = rank_fuzzy(q, cands, out.iter().map(|c| c.id).collect(), limit - out.len());
            for id in ids {
                let sql = format!("SELECT {CH_COLS} FROM channels WHERE id = ?1");
                let mut st = self.conn.prepare_cached(&sql)?;
                if let Some(r) = st.query_map([id], channel_row)?.next().transpose()? {
                    out.push(r);
                }
            }
        }
        Ok(out)
    }

    fn search_programmes(&self, q: &str, limit: usize) -> Result<Vec<ProgrammeRow>> {
        let qlen = q.chars().count();
        let mut out: Vec<ProgrammeRow> = Vec::new();
        if qlen < 3 {
            return Ok(out);
        }
        let sql = format!(
            "SELECT p.id, p.channel, p.start, p.stop, p.title, p.descr
             FROM programme_fts f JOIN programmes p ON p.id = f.rowid
             WHERE programme_fts MATCH ?1 ORDER BY rank LIMIT ?2"
        );
        {
            let mut st = self.conn.prepare_cached(&sql)?;
            let rows = st.query_map(params![phrase(q), limit as i64], programme_row)?;
            for r in rows {
                out.push(r?);
            }
        }
        if out.len() < limit.min(5) && qlen >= 4 {
            let cands = self.fuzzy_candidates("programme_fts", "programmes", "title_norm", q, 300)?;
            let ids = rank_fuzzy(q, cands, out.iter().map(|p| p.id).collect(), limit - out.len());
            for id in ids {
                let sql = format!("SELECT {PR_COLS} FROM programmes WHERE id = ?1");
                let mut st = self.conn.prepare_cached(&sql)?;
                if let Some(r) = st.query_map([id], programme_row)?.next().transpose()? {
                    out.push(r);
                }
            }
        }
        Ok(out)
    }

    /// Pull rows sharing *any* trigram with the query, best-ranked first.
    fn fuzzy_candidates(
        &self,
        fts: &str,
        content: &str,
        col: &str,
        q: &str,
        cap: usize,
    ) -> Result<Vec<(i64, String)>> {
        let grams = trigrams(q);
        if grams.is_empty() {
            return Ok(Vec::new());
        }
        let expr = grams.iter().map(|g| phrase(g)).collect::<Vec<_>>().join(" OR ");
        let sql = format!(
            "SELECT c.id, c.{col} FROM {fts} f JOIN {content} c ON c.id = f.rowid
             WHERE {fts} MATCH ?1 ORDER BY rank LIMIT ?2"
        );
        let mut st = self.conn.prepare_cached(&sql)?;
        let rows = st.query_map(params![expr, cap as i64], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }
}

/// Quote a string as an FTS5 phrase.
fn phrase(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

fn trigrams(s: &str) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    let mut out: Vec<String> = Vec::new();
    for w in chars.windows(3) {
        let g: String = w.iter().collect();
        if !out.contains(&g) {
            out.push(g);
        }
    }
    out
}

/// Keep candidates within the edit budget, nearest first, skipping `already`.
fn rank_fuzzy(q: &str, cands: Vec<(i64, String)>, already: Vec<i64>, want: usize) -> Vec<i64> {
    let pat: Vec<char> = q.chars().collect();
    let budget = match pat.len() {
        0..=3 => 0,
        4..=5 => 1,
        6..=9 => 2,
        _ => 3,
    };
    let mut scored: Vec<(usize, usize, i64)> = cands
        .into_iter()
        .filter(|(id, _)| !already.contains(id))
        .filter_map(|(id, text)| {
            let t: Vec<char> = text.chars().collect();
            let d = approx_substring_distance(&pat, &t);
            (d <= budget).then_some((d, t.len(), id))
        })
        .collect();
    scored.sort();
    scored.into_iter().take(want).map(|(_, _, id)| id).collect()
}

/// Minimum edit distance between `pattern` and any substring of `text`
/// (Sellers' algorithm: Levenshtein with a free start position in the text).
fn approx_substring_distance(pattern: &[char], text: &[char]) -> usize {
    let m = pattern.len();
    let mut prev: Vec<usize> = (0..=m).collect();
    let mut cur = vec![0usize; m + 1];
    let mut best = prev[m];
    for &tc in text {
        cur[0] = 0;
        for i in 1..=m {
            let cost = usize::from(pattern[i - 1] != tc);
            cur[i] = (prev[i - 1] + cost).min(prev[i] + 1).min(cur[i - 1] + 1);
        }
        best = best.min(cur[m]);
        std::mem::swap(&mut prev, &mut cur);
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(tvg: &str, name: &str, number: Option<u32>, url: &str) -> Channel {
        Channel {
            tvg_id: tvg.into(),
            name: name.into(),
            number,
            url: url.into(),
            ..Default::default()
        }
    }

    fn sample() -> Store {
        let mut s = Store::open_memory().unwrap();
        s.import_channels(vec![
            ch("hn", "Harbor News 24", Some(101), "http://s/1"),
            ch("ms1", "Meridian Sports 1", Some(201), "http://s/2"),
            ch("ar1", "المرفأ نيوز ٢٤", Some(301), "http://s/3"),
            ch("dm", "Delta Music", Some(601), "http://s/4"),
        ])
        .unwrap();
        s
    }

    #[test]
    fn exact_and_substring_search() {
        let s = sample();
        let r = s.search("meridian", 10).unwrap();
        assert_eq!(r.channels.len(), 1);
        assert_eq!(r.channels[0].name, "Meridian Sports 1");
        // substring in the middle of a word
        assert_eq!(s.search("sport", 10).unwrap().channels.len(), 1);
    }

    #[test]
    fn arabic_spelling_variants_match() {
        let s = sample();
        // typed without the hamza, with Arabic-Indic digits
        let r = s.search("المرفا", 10).unwrap();
        assert_eq!(r.channels.first().map(|c| c.number), Some(Some(301)));
        assert_eq!(s.search("نيوز ٢٤", 10).unwrap().channels.len(), 1);
    }

    #[test]
    fn typos_are_tolerated() {
        let s = sample();
        let r = s.search("harbr", 10).unwrap(); // missing letter
        assert_eq!(r.channels.first().map(|c| c.name.as_str()), Some("Harbor News 24"));
        let r = s.search("meridan", 10).unwrap();
        assert_eq!(r.channels.first().map(|c| c.number), Some(Some(201)));
        // nonsense stays empty
        assert!(s.search("zzzzzz", 10).unwrap().channels.is_empty());
    }

    #[test]
    fn zap_wraps_both_ways() {
        let s = sample();
        assert_eq!(s.zap(101, Direction::Up).unwrap().unwrap().number, Some(201));
        assert_eq!(s.zap(601, Direction::Up).unwrap().unwrap().number, Some(101));
        assert_eq!(s.zap(101, Direction::Down).unwrap().unwrap().number, Some(601));
        assert_eq!(s.channel_by_number(301).unwrap().unwrap().name, "المرفأ نيوز ٢٤");
        assert!(s.channel_by_number(999).unwrap().is_none());
    }

    #[test]
    fn keyset_paging_walks_everything_once() {
        let s = sample();
        let mut seen = Vec::new();
        let mut cursor = None;
        loop {
            let page = s.channels_after(cursor, 3).unwrap();
            if page.is_empty() {
                break;
            }
            let last = page.last().unwrap();
            cursor = Some((last.number.unwrap(), last.id));
            seen.extend(page.into_iter().map(|c| c.number.unwrap()));
        }
        assert_eq!(seen, vec![101, 201, 301, 601]);
    }

    #[test]
    fn favorites_survive_reimport_and_stale_channels_go() {
        let mut s = sample();
        let id = s.channel_by_number(201).unwrap().unwrap().id;
        s.set_favorite(id, true).unwrap();
        // re-import without Delta Music
        s.import_channels(vec![
            ch("hn", "Harbor News 24", Some(101), "http://s/1"),
            ch("ms1", "Meridian Sports 1", Some(201), "http://s/2"),
            ch("ar1", "المرفأ نيوز ٢٤", Some(301), "http://s/3"),
        ])
        .unwrap();
        assert_eq!(s.channel_count().unwrap(), 3);
        let favs = s.favorites().unwrap();
        assert_eq!(favs.len(), 1);
        assert_eq!(favs[0].name, "Meridian Sports 1");
        assert!(s.search("delta", 10).unwrap().channels.is_empty());
    }

    #[test]
    fn now_next_and_guide_window() {
        let mut s = sample();
        let p = |a: i64, b: i64, t: &str| Programme {
            channel: "hn".into(),
            start: a,
            stop: b,
            title: t.into(),
            desc: String::new(),
        };
        s.import_programmes(vec![p(0, 100, "Evening Bulletin"), p(100, 200, "The Long Table"), p(200, 300, "Markets Tonight")])
            .unwrap();
        let (now, next) = s.now_next("hn", 150).unwrap();
        assert_eq!(now.unwrap().title, "The Long Table");
        assert_eq!(next.unwrap().title, "Markets Tonight");
        // gap before the first programme: nothing on now, next is the first
        let (now, next) = s.now_next("hn", -10).unwrap();
        assert!(now.is_none());
        assert_eq!(next.unwrap().title, "Evening Bulletin");
        assert_eq!(s.guide_window("hn", 90, 210).unwrap().len(), 3);
        // programme search
        assert_eq!(s.search("long tab", 10).unwrap().programmes.len(), 1);
    }
}
