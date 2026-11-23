# TALLY (working name)

An original IPTV player, built to compete with IBO Player and IPTV Smarters Pro.
Windows first; the Rust core is written to be reused by a native Apple client later.

> The name is a placeholder. "Tally" is also accounting software, so pick a name and run a trademark check before release.

## Stack

| Layer | Choice | Why |
|---|---|---|
| Core | Rust (`crates/tally-core`) | Playlist/EPG parsing, SQLite store, search, Xtream client. Fast, memory-safe, reusable over FFI (UniFFI → Swift). |
| UI | Svelte 5 + TypeScript + Vite (`app/`) | Full design freedom, real Arabic/RTL, testable in a browser. |
| Shell | Tauri 2 (planned) | Small Windows installer, native WebView2. |
| Playback | libmpv (planned) | Hardware decoding, every IPTV codec and container. |

## Status

- `tally-core`: done and tested (20 tests). Benchmark on 100k channels: first 50 rows in 0.5 ms, search 3–10 ms, import ~0.6 s, ~100 MB peak.
- `app/`: full UI prototype on mock data: Home, Live TV (channels, guide, multiview), Movies, Series, Replay, Playlists, Settings, Player, Search. English and Arabic (RTL), keyboard/remote navigation.
- Not yet: Tauri shell, libmpv embedding, real Xtream provider test.

## Run

```
cd app
npm install
npm run dev        # http://localhost:5173
npm run check      # types + i18n key parity
```

```
cargo test --workspace
cargo run --release -p tally-core --example bench
```

## Credentials

Xtream credentials are never stored in the repo. The example importer reads them from environment variables:

```
$env:TALLY_HOST = "http://provider.example"
$env:TALLY_USER = "your-user"
$env:TALLY_PASS = "your-password"
cargo run --release -p tally-core --example xtream_import
```

In the app they will be kept in the Windows Credential Manager. Errors scrub the username and password.

## Design

Graphite background, paper-white text, one accent (changeable), flat procedural poster art with no gradients, condensed display type, IBM Plex Sans / Plex Sans Arabic. Everything is reachable by arrow keys, so it works with a mouse, keyboard or TV-style remote.

## Desktop build

The Windows installer is built by CI (`windows` job) and uploaded as the `tally-windows` artifact on every push to a branch. Download it from the Actions tab, no local Rust toolchain needed.

Local: `cd app && npm install`, then from the repo root `npm exec --prefix app -- tauri build`.

Next: libmpv playback (transparent window with mpv rendering behind the UI), keychain storage for credentials, guide import.
