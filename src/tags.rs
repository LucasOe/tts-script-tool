use crate::error::{Error, Result};
use derive_more::{Display, Index, IntoIterator};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Deserialize, Serialize, Clone, Debug, IntoIterator, Index)]
pub struct Tags(Vec<Tag>);

impl Tags {
    pub fn add(&mut self, tag: Tag) {
        self.0.push(tag);
    }

    pub fn to_json_string(&self) -> Result<String> {
        serde_json::to_string(&self.0).map_err(Error::SerdeError)
    }

    /// Tags that follow the "scripts/<File>.ttslua" naming convention are valid
    pub fn filter_valid(&self) -> Self {
        Self(
            self.clone()
                .into_iter()
                .filter(|tag| tag.is_valid())
                .collect::<Vec<Tag>>(),
        )
    }

    /// Tags that don't follow the "scripts/<File>.ttslua" naming convention are invalid
    pub fn filter_invalid(&self) -> Self {
        Self(
            self.clone()
                .into_iter()
                .filter(|tag| !tag.is_valid())
                .collect::<Vec<Tag>>(),
        )
    }

    /// Get a valid tag
    pub fn valid(&self) -> Result<Option<Tag>> {
        let valid = self.filter_valid();
        match valid.0.len() {
            1 => Ok(valid.0.get(0).cloned()),
            0 => Ok(None),
            _ => Err("{guid} has multiple valid script tags: {tags:?}".into()),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, Display)]
pub struct Tag(String);

impl Tag {
    /// Create a new tag using `scripts/{file_name}` as a name
    pub fn from(path: &Path) -> Self {
        let file_name = path.file_name().unwrap().to_str().unwrap();
        Self(format!("scripts/{file_name}"))
    }

    /// Tags that follow the "scripts/<File>.ttslua" naming convention are valid
    pub fn is_valid(&self) -> bool {
        let exprs = regex::Regex::new(r"^(scripts/)[\d\w]+(\.lua|\.ttslua)$").unwrap();
        exprs.is_match(&self.0)
    }

    pub fn read_file(&self, path: &Path) -> Result<String> {
        let file_name = Path::new(&self.0).file_name().unwrap();
        let file_path = String::from(path.join(file_name).to_string_lossy());
        std::fs::read_to_string(file_path).map_err(Error::Io)
    }
}
