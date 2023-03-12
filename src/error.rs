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
    ValidTags { guid: String, tags: Vec<String> },
    #[error("{0} does not exist")]
    MissingGuid(String),
}

pub type Result<T> = std::result::Result<T, Error>;
