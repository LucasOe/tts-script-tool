use std::{ffi::OsStr, path::PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("not a valid GUID")]
    InvalidGUID,
    #[error("not a file")]
    NotAFile,
    #[error("does not exist")]
    DoesNotExist,
    #[error("not a json file")]
    NotJsonFile,
}

pub fn guid(s: &str) -> Result<String, ParseError> {
    let len = s.len();
    let is_numerical = s.chars().all(|c| c.is_ascii_alphanumeric());
    match (len, is_numerical) {
        (6, true) => Ok(s.into()),
        _ => Err(ParseError::InvalidGUID),
    }
}

pub fn path_is_file(s: &str) -> Result<PathBuf, ParseError> {
    let path = PathBuf::from(s);
    match path.is_file() {
        true => Ok(path),
        false => Err(ParseError::NotAFile),
    }
}

pub fn path_exists(s: &str) -> Result<PathBuf, ParseError> {
    let path = PathBuf::from(s);
    match path.exists() {
        true => Ok(path),
        false => Err(ParseError::DoesNotExist),
    }
}

pub fn path_is_json(s: &str) -> Result<PathBuf, ParseError> {
    let path = PathBuf::from(s);
    match path.extension() == Some(OsStr::new("json")) {
        true => Ok(path),
        false => Err(ParseError::NotJsonFile),
    }
}
