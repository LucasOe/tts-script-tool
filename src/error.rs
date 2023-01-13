use thiserror::Error;
use tts_external_api::Value;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    InquireError(#[from] inquire::InquireError),
    #[error("{guid} has multiple valid script tags: {tags:?}")]
    ValidTags { guid: String, tags: Vec<Value> },
    #[error("{0} does not exist")]
    MissingGuid(String),
}

pub type Result<T> = std::result::Result<T, Error>;
