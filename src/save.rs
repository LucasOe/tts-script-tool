//! Struct definitions for the Tabletop Simulator [Save File Format](https://kb.tabletopsimulator.com/custom-content/save-file-format/).

use std::{collections::HashMap, path::PathBuf};
use std::{fs, io};

use crate::error::{Error, Result};
use crate::objects::Objects;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tts_external_api::ExternalEditorApi;

/// Holds a state of the game
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Save {
    #[serde(rename = "SaveName")]
    pub name: String,
    #[serde(rename = "LuaScript", default)]
    pub lua_script: String,
    #[serde(rename = "XmlUI", default)]
    pub xml_ui: String,
    #[serde(rename = "ObjectStates")]
    pub objects: Objects,

    // Other fields that are not relevant
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

impl Save {
    /// Reads the currently open save file and returns it as a `Save`.
    pub fn read_save(api: &ExternalEditorApi) -> Result<Self> {
        let path = PathBuf::from(api.get_scripts()?.save_path);
        let file = fs::File::open(path)?;
        let reader = io::BufReader::new(file);

        serde_json::from_reader(reader).map_err(Error::SerdeError)
    }

    /// Writes this `Save` to the currently open save file.
    pub fn write_save(&self, api: &ExternalEditorApi) -> Result<&Self> {
        let path = PathBuf::from(api.get_scripts()?.save_path);
        //let path = std::path::Path::new("./out.json");
        let file = fs::File::create(path)?;
        let writer = io::BufWriter::new(file);

        serde_json::to_writer_pretty(writer, self).map_err(Error::SerdeError)?;

        Ok(self) // Return Self for method chaining
    }
}
