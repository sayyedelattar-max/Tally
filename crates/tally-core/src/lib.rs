//! Tally core: everything the player UI needs that is not pixels or video.
//!
//! * [`m3u`]   streaming M3U / M3U8 playlist parser
//! * [`xmltv`] streaming XMLTV guide parser (never loads the whole file)
//! * [`store`] SQLite (WAL) store with trigram full-text search
//! * [`norm`]  Arabic-aware text normalisation shared by import and search
//!
//! The UI layer talks to [`store::Store`]; playback is handled separately by libmpv.

pub mod error;
pub mod library;
pub mod m3u;
pub mod model;
pub mod norm;
pub mod proxy;
pub mod store;
pub mod xmltv;
pub mod xtream;

pub use error::{Error, Result};
pub use model::{Channel, Programme};
pub use store::Store;
