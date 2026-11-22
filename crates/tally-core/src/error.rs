use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("xml error: {0}")]
    Xml(String),
    /// Network failure. The message never contains the account password.
    #[error("network error: {0}")]
    Http(String),
    /// The server answered but refused the login or sent something unusable.
    #[error("account error: {0}")]
    Account(String),
}

pub type Result<T> = std::result::Result<T, Error>;
