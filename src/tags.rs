use crate::error::Result;
use derive_more::{Deref, DerefMut, Display, IntoIterator};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A list of [`Tags`](Tag) associated with an [`Object`](crate::objects::Object).
/// Tags can be filtered by valid an invalid tags.
#[derive(Deserialize, Serialize, Default, Clone, Debug, IntoIterator, Deref, DerefMut)]
pub struct Tags(Vec<Tag>);

impl From<Vec<Tag>> for Tags {
    fn from(vec: Vec<Tag>) -> Self {
        Tags(vec)
    }
}

impl FromIterator<Tag> for Tags {
    fn from_iter<I: IntoIterator<Item = Tag>>(iter: I) -> Self {
        Tags(iter.into_iter().collect::<Vec<Tag>>())
    }
}

impl Tags {
    /// Consumes `Tags`, returning the wrapped value.
    pub fn into_inner(self) -> Vec<Tag> {
        self.0
    }

    /// Returns a valid [`Tag`], if the list only contains a single valid tag.
    /// If it contains no valid Tags it returns [`None`].
    /// If the list contains multiple valid tags, this function returns an [`Error::Msg`].
    pub fn valid(&self) -> Result<Option<&Tag>> {
        let valid: Vec<&Tag> = self.iter().filter(|tag| tag.is_valid()).collect();
        match valid.len() {
            0 | 1 => Ok(valid.get(0).cloned()),
            _ => Err("{guid} has multiple valid script tags: {tags}".into()),
        }
    }
}

/// A tag associated with an [`Object`](crate::objects::Object).
#[derive(Deserialize, Serialize, Clone, Debug, Display)]
pub struct Tag(String);

impl From<&PathBuf> for Tag {
    /// Create a new tag using `scripts/<FileName>.ttslua` as a name.
    fn from(path: &PathBuf) -> Self {
        let file_name = path.file_name().unwrap();
        Self(format!("scripts/{}", file_name.to_str().unwrap()))
    }
}

impl Tag {
    /// Tags that follow the `scripts/<File>.ttslua` naming convention are valid.
    pub fn is_valid(&self) -> bool {
        let exprs = regex::Regex::new(r"^(scripts/)[\d\w]+(\.lua|\.ttslua)$").unwrap();
        exprs.is_match(&self.0)
    }

    /// Joins the file name from `self` and parent directory from `path`.
    pub fn join_path(&self, path: &Path) -> Result<PathBuf> {
        if !self.is_valid() {
            return Err("Invalid Tag: {self}".into());
        }

        // Use the parent directory if a file path is provided as an input
        let path_dir = match path.is_file() {
            true => path.parent().unwrap(),
            false => path,
        };
        // Only use the file name from the [`Tag`]
        let file_name = Path::new(&self.0).file_name().unwrap();

        Ok(path_dir.join(file_name))
    }

    /// Returns true if the tag is equal to the path
    pub fn is_path(&self, path: &Path) -> bool {
        let tag_name = Path::new(&self.0).file_name().unwrap();
        let path_name = path.file_name().unwrap();
        tag_name == path_name
    }
}
