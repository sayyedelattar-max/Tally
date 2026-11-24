/// A channel as it appears in a playlist.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Channel {
    /// Stable identity from the provider (e.g. `x:101`), so favorites survive when addresses change.
    /// Empty means "derive one from the guide id or the address".
    pub key: String,
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

/// A movie or a series from the provider's on-demand catalogue.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Title {
    /// `movie` or `series`.
    pub kind: String,
    /// The provider's id for it.
    pub ext_id: String,
    pub name: String,
    pub group: String,
    pub poster: String,
    pub plot: String,
    pub rating: f64,
    pub year: String,
    /// Unix seconds when the provider added it (0 if unknown).
    pub added: i64,
    /// Container the file comes in (`mp4`, `mkv`…). Empty for series.
    pub ext: String,
}

/// One episode of a series.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Episode {
    pub id: String,
    pub season: u32,
    pub number: u32,
    pub title: String,
    pub plot: String,
    pub duration: String,
    pub image: String,
    pub ext: String,
}

/// A series with its episodes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SeriesDetail {
    pub name: String,
    pub plot: String,
    pub poster: String,
    pub episodes: Vec<Episode>,
}
