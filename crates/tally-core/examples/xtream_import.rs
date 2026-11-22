//! Import a real Xtream Codes subscription into a local Tally database.
//!
//! Run it on YOUR machine so the credentials never leave it:
//!
//!   PowerShell:
//!     $env:TALLY_URL  = "http://provider.example"
//!     $env:TALLY_USER = "your-username"
//!     $env:TALLY_PASS = "your-password"
//!     cargo run --release -p tally-core --example xtream_import
//!     Remove-Item Env:TALLY_PASS          # clear it afterwards
//!
//!   bash:
//!     TALLY_URL=http://provider.example TALLY_USER=... TALLY_PASS=... \
//!       cargo run --release -p tally-core --example xtream_import
//!
//! Options: `--no-guide` skips the XMLTV download. `TALLY_DB` sets the database path
//! (default `./tally.db`). Nothing is printed that contains the password or a stream URL.

use std::collections::HashMap;
use std::sync::mpsc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tally_core::{Channel, Error, Programme, Store, xtream::XtreamClient};

fn need(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| {
        eprintln!("missing environment variable {name} (see the comment at the top of this file)");
        std::process::exit(2);
    })
}

/// Unix seconds -> "YYYY-MM-DD" (UTC).
fn date(ts: i64) -> String {
    let z = ts.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

fn main() {
    let skip_guide = std::env::args().any(|a| a == "--no-guide");
    let client = XtreamClient::new(&need("TALLY_URL"), &need("TALLY_USER"), &need("TALLY_PASS"));
    let db = std::env::var("TALLY_DB").unwrap_or_else(|_| "tally.db".into());

    // 1. Account
    print!("checking account... ");
    let acct = match client.account() {
        Ok(a) => a,
        Err(e) => {
            println!("failed\n{e}");
            std::process::exit(1);
        }
    };
    println!("ok");
    println!("  status:        {}", acct.status);
    if let Some(exp) = acct.expires_at {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
        println!("  expires:       {} ({} days left)", date(exp), (exp - now).div_euclid(86_400));
    }
    if let Some(max) = acct.max_connections {
        println!("  connections:   {} allowed, {} in use", max, acct.active_connections.unwrap_or(0));
    }
    if !acct.formats.is_empty() {
        println!("  formats:       {}", acct.formats.join(", "));
    }
    println!("  trial account: {}", if acct.is_trial { "yes" } else { "no" });

    // 2. Playlist
    let t = Instant::now();
    print!("\ndownloading playlist... ");
    let mut channels: Vec<Channel> = Vec::new();
    match client.playlist(|c| channels.push(c)) {
        Ok(n) => println!("{n} channels in {:.1}s", t.elapsed().as_secs_f64()),
        Err(e) => {
            println!("failed\n{e}");
            std::process::exit(1);
        }
    }
    let mut groups: HashMap<String, usize> = HashMap::new();
    for c in &channels {
        *groups.entry(c.group.clone()).or_default() += 1;
    }
    let mut top: Vec<_> = groups.into_iter().collect();
    top.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    println!("  {} groups; largest:", top.len());
    for (g, n) in top.iter().take(8) {
        println!("    {n:>6}  {}", if g.is_empty() { "(no group)" } else { g });
    }
    println!("  first few names:");
    for c in channels.iter().take(5) {
        println!("    {}", c.name);
    }

    let mut store = Store::open(std::path::Path::new(&db)).expect("open database");
    let t = Instant::now();
    let n = store.import_channels(channels).expect("store channels");
    println!("  stored {n} channels + search index in {:.1}s", t.elapsed().as_secs_f64());

    // 3. Guide, parsed on one thread and stored on another so memory stays flat
    if skip_guide {
        println!("\nskipping guide (--no-guide)");
    } else {
        let t = Instant::now();
        print!("\ndownloading guide... ");
        let (tx, rx) = mpsc::sync_channel::<Result<Programme, Error>>(4096);
        let client_ref = &client;
        let stored = std::thread::scope(|s| {
            s.spawn(move || {
                let err_tx = tx.clone();
                let r = client_ref.guide(|p| {
                    let _ = tx.send(Ok(p));
                });
                if let Err(e) = r {
                    let _ = err_tx.send(Err(e)); // makes the store roll back
                }
            });
            store.try_import_programmes(rx)
        });
        match stored {
            Ok(n) => println!("{n} programmes stored in {:.1}s", t.elapsed().as_secs_f64()),
            Err(e) => println!("failed, previous guide kept\n{e}"),
        }
    }

    println!("\ndone. database: {db}");
    println!("channels in database: {}", store.channel_count().unwrap_or(0));
}
