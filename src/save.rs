//! Struct definitions for the Tabletop Simulator [Save File Format](https://kb.tabletopsimulator.com/custom-content/save-file-format/).

use std::{collections::HashMap, path::PathBuf};
use std::{fs, io};

use crate::error::{Error, Result};
use crate::tags::Tags;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tts_external_api::ExternalEditorApi;

/// Holds a state of the game
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Save {
    #[serde(rename = "SaveName")]
    pub save_name: String,
    #[serde(rename = "LuaScript", default)]
    pub lua_script: String,
    #[serde(rename = "XmlUI", default)]
    pub xml_ui: String,
    #[serde(rename = "ObjectStates")]
    pub object_states: Vec<Object>,

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

    /// Adds the objects to the existing objects in this `Save`.
    /// Objects with the same guid will be replaced.
    pub fn add_objects(&mut self, objects: &[Object]) -> Result<&Self> {
        for object_state in &mut self.object_states {
            if let Some(object) = objects.iter().find(|object| object == &object_state) {
                *object_state = object.clone();
            };
        }

        Ok(self) // Return Self for method chaining
    }

    pub fn find_object(self, guid: &String) -> Result<Object> {
        self.object_states
            .into_iter()
            .find(|object| object.has_guid(guid))
            .ok_or("{guid} does not exist".into())
    }
}

/// An object loaded in the current save or savestate.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Object {
    #[serde(rename = "GUID")]
    pub guid: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Nickname", default)]
    pub nickname: String,
    #[serde(rename = "Tags", default)]
    pub tags: Tags,
    #[serde(rename = "LuaScript", default)]
    pub lua_script: String,
    #[serde(rename = "XmlUI", default)]
    pub xml_ui: String,

    // Other fields that are not relevant
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

impl std::fmt::Display for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match (self.nickname.is_empty(), self.name.is_empty()) {
            (true, true) => write!(f, "'{}'", self.guid),
            (true, false) => write!(f, "'{}' ({})", self.guid, self.name),
            _ => write!(f, "'{}' ({})", self.guid, self.nickname),
        }
    }
}

impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        self.guid == other.guid
    }
}

impl Object {
    /// Return `true` if the object has the same guid
    pub fn has_guid(&self, guid: &String) -> bool {
        &self.guid == guid
    }

    /// Create a Value used for the [`reload!`] macro.
    pub fn to_value(&self) -> Value {
        serde_json::json!({
            "guid": self.guid,
            "script": self.lua_script,
            "ui": self.xml_ui,
        })
    }
}
