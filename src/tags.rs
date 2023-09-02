use crate::error::{Error, Result};
use colored::*;
use derive_more::{Deref, DerefMut, Display, IntoIterator};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A list of [`Tags`](Tag) associated with an [`Object`](crate::objects::Object).
/// Tags can be filtered by valid an invalid tags.
#[derive(Deserialize, Serialize, Default, Clone, Debug, IntoIterator, Deref, DerefMut, Display)]
#[display(fmt = "{}", "self.0.iter().format(\", \")")]
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
}

/// A tag associated with an [`Object`](crate::objects::Object).
#[derive(Deserialize, Serialize, Clone, Debug, Display, PartialEq)]
#[display(fmt = "{}", "self.0.yellow()")]
pub struct Tag(String);

impl TryFrom<&Path> for Tag {
    type Error = Error;

    /// Create a new tag from a path, using `scripts/<FilePath>.lua` and `ui/<FilePath>.xml` as a naming convention.
    fn try_from(path: &Path) -> Result<Self> {
        use path_slash::PathExt as _;

        // Note: `strip_prefix("./")` doesn't remove `.\\` on linux
        let file_path = match path.strip_prefix("./") {
            Ok(file_path) => file_path.to_slash_lossy(), // Replace `\` with `/`
            Err(_) => return Err("Path has to be relative".into()),
        };

        let file_ext = match path.extension() {
            Some(file_ext) => file_ext.to_str().unwrap(),
            None => return Err("Path must end in a file extension".into()),
        };

        match file_ext {
            "lua" | "ttslua" => Ok(Self(format!("lua/{}", file_path))),
            "xml" => Ok(Self(format!("xml/{}", file_path))),
            _ => Err("Path is not a lua or xml file".into()),
        }
    }
}

impl<P: AsRef<Path>> PartialEq<P> for Tag {
    fn eq(&self, other: &P) -> bool {
        match Tag::try_from(other.as_ref()) {
            Ok(tag) => self.0 == tag.0,
            Err(_) => false,
        }
    }
}

impl Tag {
    /// Returns `true` if either `is_lua` or `is_xml` returns true.
    pub fn is_valid(&self) -> bool {
        self.is_lua() || self.is_xml()
    }

    /// Returns `true` if `self` follows the `lua/<FilePath>.lua` naming convention.
    pub fn is_lua(&self) -> bool {
        let exprs = regex::Regex::new(r"^lua/.+(\.lua|\.ttslua)$").unwrap();
        exprs.is_match(&self.0)
    }

    /// Returns `true` if `self` follows the `xml/<FilePath>.xml` naming convention.
    pub fn is_xml(&self) -> bool {
        let exprs = regex::Regex::new(r"^xml/.+(\.xml)$").unwrap();
        exprs.is_match(&self.0)
    }

    /// Returns `self` as a path if it is valid.
    /// `lua/foo/bar.lua` would return `foo/bar.lua`.
    pub fn path(&self) -> Result<&Path> {
        let path = Path::new(&self.0);
        match self {
            _ if self.is_lua() => Ok(path.strip_prefix("lua/").unwrap()),
            _ if self.is_xml() => Ok(path.strip_prefix("xml/").unwrap()),
            _ => Err("{self} is not a valid tag".into()),
        }
    }

    /// Joins the file name from `self` and parent directory from `path`.
    /// If `path` is a file, `path` gets returned instead without modification.
    pub fn join_path<P: AsRef<Path>>(&self, path: &P) -> Result<PathBuf> {
        let tag_path = self.path()?;
        let full_path = path.as_ref().join(tag_path);
        match full_path.is_file() {
            true => Ok(full_path),
            false => Err(format!("{} is not a file", full_path.display()).into()),
        }
    }
}
