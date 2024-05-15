use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
    #[error(transparent)]
    StripPrefixError(#[from] std::path::StripPrefixError),
    #[error("{0}")]
    Msg(String),
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error::Msg(s.into())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Msg(s)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
