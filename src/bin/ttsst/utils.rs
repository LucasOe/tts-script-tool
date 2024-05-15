use anyhow::Result;
use itertools::Itertools;
use std::path::{Path, PathBuf};

pub trait Reduce<P> {
    /// Filters and deduplicates the collection of paths, returning a new collection.
    ///
    /// This method removes duplicate paths based on their logical content and ensures that
    /// subfolders are not included if a parent folder is present in the collection.
    fn reduce<T: FromIterator<P>>(&self) -> T;
}

impl<U: AsRef<[P]>, P: AsRef<Path> + Clone> Reduce<P> for U {
    fn reduce<T: FromIterator<P>>(&self) -> T {
        self.as_ref()
            .iter()
            .unique_by(|path| path.as_ref().to_owned())
            .filter(|&this| {
                !self.as_ref().iter().any(|other| {
                    let paths = (this.as_ref(), other.as_ref());
                    paths.0 != paths.1 && paths.0.starts_with(paths.1)
                })
            })
            .cloned()
            .collect()
    }
}

pub trait StripCurrentDir {
    fn strip_current_dir(&self) -> Result<PathBuf>;
}

impl StripCurrentDir for PathBuf {
    fn strip_current_dir(&self) -> Result<PathBuf> {
        let path = self.strip_prefix(std::env::current_dir()?)?;
        Ok(PathBuf::from(".\\").join(path))
    }
}
