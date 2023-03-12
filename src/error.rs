use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    InquireError(#[from] inquire::InquireError),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
    #[error("{guid} has multiple valid script tags: {tags:?}")]
    TooManyTags { guid: String, tags: Vec<String> },
    #[error("{guid} does not exist")]
    MissingGuid { guid: String },
}

pub type Result<T> = std::result::Result<T, Error>;
