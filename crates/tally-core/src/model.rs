/// A channel as it appears in a playlist.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Channel {
    pub tvg_id: String,
    pub name: String,
    pub group: String,
    pub logo: String,
    pub url: String,
    /// Channel number from the playlist (`tvg-chno`), if it has one.
    pub number: Option<u32>,
}

/// One guide entry. Times are Unix seconds (UTC).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Programme {
    /// The XMLTV channel id; matches [`Channel::tvg_id`].
    pub channel: String,
    pub start: i64,
    pub stop: i64,
    pub title: String,
    pub desc: String,
}
