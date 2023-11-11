use std::{collections::HashMap, path::PathBuf};
use std::{fs, io};

use log::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tts_external_api::ExternalEditorApi;

use crate::error::Result;
use crate::objects::Objects;
use crate::tags::Label;
use crate::Tag;

#[derive(Deserialize, Serialize, Debug)]
pub struct ComponentTags {
    pub labels: Vec<Label>,
}

impl ComponentTags {
    // Add `tag` to `self`, if it isn't already included in the labels
    pub fn push(&mut self, tag: Tag) {
        let label = Label::from(tag.clone());
        if !self.labels.contains(&label) {
            self.labels.push(label);
            info!("added {} as a component tag", tag);
        }
    }

    // Remove component tags that exist as object tags
    pub fn remove_object_tags(&mut self, objects: &Objects) {
        self.labels.retain(|label| {
            !objects.iter().any(|object| {
                object
                    .tags
                    .iter()
                    .any(|tag| &Label::from(tag.clone()) == label)
            })
        })
    }
}

/// A representation of the Tabletop Simulator [Save File Format](https://kb.tabletopsimulator.com/custom-content/save-file-format/).
#[derive(Deserialize, Serialize, Debug)]
pub struct Save {
    #[serde(rename = "SaveName")]
    pub name: String,
    #[serde(rename = "LuaScript", default)]
    pub lua_script: String,
    #[serde(rename = "XmlUI", default)]
    pub xml_ui: String,
    #[serde(rename = "ObjectStates")]
    pub objects: Objects,
    #[serde(rename = "ComponentTags")]
    pub tags: ComponentTags,

    // Other fields
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

impl Save {
    /// Reads the currently open save file and returns it as a `Save`.
    pub fn read(api: &ExternalEditorApi) -> Result<Self> {
        let save_path = PathBuf::from(api.get_scripts()?.save_path);
        let file = fs::File::open(&save_path)?;
        let reader = io::BufReader::new(file);

        debug!("trying to read save from {}", save_path.display());
        serde_json::from_reader(reader).map_err(|err| err.into())
    }

    /// Writes `self` to the save file that is currently loaded ingame.
    ///
    /// If `self` contains an empty `lua_script` or `xml_ui` string,
    /// the function will cause a connection error.
    pub fn write(&self, api: &ExternalEditorApi) -> Result<()> {
        let save_path = PathBuf::from(api.get_scripts()?.save_path);
        let file = fs::File::create(&save_path)?;
        let writer = io::BufWriter::new(file);

        debug!("trying to write save to {}", save_path.display());
        serde_json::to_writer_pretty(writer, self).map_err(|err| err.into())
    }
}
