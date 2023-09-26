use anyhow::{bail, Result};
use std::{ffi::OsStr, path::PathBuf};

pub fn guid(s: &str) -> Result<String> {
    let len = s.len();
    let num = s.chars().all(|c| c.is_ascii_alphanumeric());
    match (len, num) {
        (6, true) => Ok(s.to_string()),
        _ => bail!("not a valid guid"),
    }
}

pub fn path_is_file(s: &str) -> Result<PathBuf> {
    let path = PathBuf::from(s);
    match path.is_file() {
        true => Ok(path),
        false => bail!("not a file"),
    }
}

pub fn path_exists(s: &str) -> Result<PathBuf> {
    let path = PathBuf::from(s);
    match path.exists() {
        true => Ok(path),
        false => bail!("does not exist"),
    }
}

pub fn path_is_json(s: &str) -> Result<PathBuf> {
    let path = PathBuf::from(s);
    match path.extension() == Some(OsStr::new("json")) {
        true => Ok(path),
        false => bail!("not a json file"),
    }
}
