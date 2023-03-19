use crate::error::{Error, Result};
use derive_more::{Deref, DerefMut, Display, IntoIterator};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A list of [`Tags`](Tag) associated with an [`Object`](crate::objects::Object).
/// Tags can be filtered by valid an invalid tags.
#[derive(Deserialize, Serialize, Default, Clone, Debug, IntoIterator, Deref, DerefMut)]
pub struct Tags(Vec<Tag>);

impl Tags {
    /// Consumes `Tags`, returning the wrapped value.
    pub fn into_inner(self) -> Vec<Tag> {
        self.0
    }

    /// Tags that follow the naming convention defined in [`Tag::is_valid()`] are valid
    pub fn filter_valid(self) -> Self {
        Self(
            self.into_iter()
                .filter(|tag| tag.is_valid())
                .collect::<Vec<Tag>>(),
        )
    }

    /// Tags that don't follow the naming convention defined in [`Tag::is_valid()`] are invalid
    pub fn filter_invalid(self) -> Self {
        Self(
            self.into_iter()
                .filter(|tag| !tag.is_valid())
                .collect::<Vec<Tag>>(),
        )
    }

    /// Returns a valid [`Tag`], if the list only contains a single valid tag.
    /// If it contains no valid Tags it returns [`None`].
    /// If the list contains multiple valid tags, this function returns an [`Error::Msg`].
    pub fn valid(self) -> Result<Option<Tag>> {
        let valid = self.filter_valid();
        match valid.0.len() {
            1 => Ok(valid.0.get(0).cloned()),
            0 => Ok(None),
            _ => Err("{guid} has multiple valid script tags: {tags}".into()),
        }
    }
}

/// A tag associated with an [`Object`](crate::objects::Object).
#[derive(Deserialize, Serialize, Clone, Debug, Display)]
pub struct Tag(String);

impl Tag {
    /// Construct a new `Tag`.
    pub fn new(inner: String) -> Self {
        Tag(inner)
    }

    /// Create a new tag using `scripts/<File>.ttslua` as a name.
    pub fn from(path: &Path) -> Self {
        let file_name = path.file_name().unwrap().to_str().unwrap();
        Self(format!("scripts/{file_name}"))
    }

    /// Tags that follow the `scripts/<File>.ttslua` naming convention are valid.
    pub fn is_valid(&self) -> bool {
        let exprs = regex::Regex::new(r"^(scripts/)[\d\w]+(\.lua|\.ttslua)$").unwrap();
        exprs.is_match(&self.0)
    }

    /// Reads the file from the tag and returns the content, if the tag is valid.
    pub fn read_file(&self, path: &Path) -> Result<String> {
        match self.is_valid() {
            true => {
                let file_name = Path::new(&self.0).file_name().unwrap();
                let file_path = String::from(path.join(file_name).to_string_lossy());
                std::fs::read_to_string(file_path).map_err(Error::Io)
            }
            false => Err("Invalid Tag: {self}".into()),
        }
    }
}
