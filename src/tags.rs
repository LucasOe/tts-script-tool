use crate::error::Result;
use derive_more::{Deref, DerefMut, Display, IntoIterator};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A list of [`Tags`](Tag) associated with an [`Object`](crate::objects::Object).
/// Tags can be filtered by valid an invalid tags.
#[derive(Deserialize, Serialize, Default, Clone, Debug, IntoIterator, Deref, DerefMut)]
pub struct Tags(Vec<Tag>);

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

    /// Tags that follow the naming convention defined in [`Tag::is_valid()`] are valid
    pub fn filter_valid(&self) -> Self {
        self.iter().filter(|tag| tag.is_valid()).cloned().collect()
    }

    /// Tags that don't follow the naming convention defined in [`Tag::is_valid()`] are invalid
    pub fn filter_invalid(&self) -> Self {
        self.iter().filter(|tag| !tag.is_valid()).cloned().collect()
    }

    /// Returns a valid [`Tag`], if the list only contains a single valid tag.
    /// If it contains no valid Tags it returns [`None`].
    /// If the list contains multiple valid tags, this function returns an [`Error::Msg`].
    pub fn valid(&self) -> Result<Option<Tag>> {
        let valid = self.filter_valid();
        match valid.len() {
            1 => Ok(valid.get(0).cloned()),
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

    /// Joins the [`Tag`] and the path provided as an argument into a full path.
    ///
    /// `scripts/<File>.ttslua` and `./src` would result in `./src/<File>.ttslua`.
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

    /// Returns true if the tag equals the path
    pub fn is_path(&self, path: &Path) -> bool {
        let tag_name = Path::new(&self.0).file_name().unwrap();
        let path_name = path.file_name().unwrap();
        tag_name == path_name
    }
}
