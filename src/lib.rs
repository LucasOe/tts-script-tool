pub mod error;
pub mod macros;
pub mod objects;
pub mod tags;

use crate::error::{Error, Result};
use serde::{de::DeserializeOwned, Serialize};

pub trait JsonObject {
    /// Converts the value to a JSON string.
    ///
    /// Newtype structs will serialize and deserialize to the inner value with no wrapper.
    /// See [Serde Json](https://serde.rs/json.html) for more information.
    fn to_json_string(&self) -> Result<String>
    where
        Self: Serialize + DeserializeOwned,
    {
        serde_json::to_string(&self).map_err(Error::SerdeError)
    }
}
