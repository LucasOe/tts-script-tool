pub mod error;
pub mod macros;
pub mod objects;
pub mod tags;

use crate::error::{Error, Result};
use serde::{de::DeserializeOwned, Serialize};

pub trait JsonObject {
    /// Converts the value to a JSON string.
    fn to_json_string(&self) -> Result<String>
    where
        Self: Serialize + DeserializeOwned,
    {
        serde_json::to_string(&self).map_err(Error::SerdeError)
    }
}
