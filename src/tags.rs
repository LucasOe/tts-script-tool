use colored::*;
use derive_more::{Deref, DerefMut, Display, IntoIterator};
use itertools::Itertools;
use path_slash::PathExt;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// A list of [`Tags`](Tag) associated with an [`Object`](crate::objects::Object).
/// Tags can be filtered by valid an invalid tags.
#[derive(Deserialize, Serialize, Default, Clone, Debug, Deref, DerefMut, Display, IntoIterator)]
#[display(fmt = "{}", "self.0.iter().format(\", \")")]
pub struct Tags(Vec<Tag>);

impl From<Vec<Tag>> for Tags {
    fn from(vec: Vec<Tag>) -> Self {
        Tags(vec)
    }
}

impl FromIterator<Tag> for Tags {
    fn from_iter<I: IntoIterator<Item = Tag>>(iter: I) -> Self {
        Tags(iter.into_iter().collect_vec())
    }
}

impl Tags {
    /// Consumes `Tags`, returning the wrapped value.
    pub fn into_inner(self) -> Vec<Tag> {
        self.0
    }
}

/// A tag associated with an [`Object`](crate::objects::Object).
#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Display)]
#[display(fmt = "{}", "self.0.yellow()")]
pub struct Tag(String);

impl TryFrom<&Path> for Tag {
    type Error = Error;

    /// Create a new tag from a path, using `scripts/<FilePath>.lua` and `ui/<FilePath>.xml` as a naming convention.
    fn try_from(path: &Path) -> Result<Self> {
        // Note: `strip_prefix` might not work on linux systems
        let file_path = match path.strip_prefix(".\\") {
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

impl Tag {
    /// Consumes `Tag`, returning the wrapped value.
    pub fn into_inner(self) -> String {
        self.0
    }

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
    /// `lua/foo/bar.lua` would return `./foo/bar.lua`.
    pub fn path(&self) -> Result<PathBuf> {
        let path = Path::new(&self.0);
        match self {
            _ if self.is_lua() => Ok(path.strip_prefix("lua/")?),
            _ if self.is_xml() => Ok(path.strip_prefix("xml/")?),
            _ => Err("{self} is not a valid tag".into()),
        }
        .map(|file| Path::new("./").join(file))
    }

    /// Determines whether `base` is a prefix of `self`.
    pub fn starts_with<P: AsRef<Path>>(&self, base: &P) -> bool {
        match self.path() {
            Ok(path) => path.starts_with(base),
            Err(_) => false,
        }
    }
}

#[derive(Deserialize, Serialize, PartialEq, Debug)]
pub struct Label {
    pub displayed: String,
    pub normalized: String,
}

impl From<Tag> for Label {
    fn from(value: Tag) -> Self {
        Label {
            displayed: value.0.clone(),
            normalized: value.0.clone(),
        }
    }
}
