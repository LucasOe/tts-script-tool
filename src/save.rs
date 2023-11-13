use std::path::Path;
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

#[derive(Debug)]
pub struct SaveFile {
    pub save: Save,
    pub path: PathBuf,
}

impl SaveFile {
    /// Reads the currently open save file and returns it as a `SaveFile`.
    pub fn read(api: &ExternalEditorApi) -> Result<Self> {
        let save_path = PathBuf::from(&api.get_scripts()?.save_path);
        SaveFile::read_from_path(save_path)
    }

    // Reads a save from a path and returns it as a `SaveFile`.
    pub fn read_from_path<P: AsRef<Path> + Into<PathBuf>>(save_path: P) -> Result<Self> {
        let file = fs::File::open(&save_path)?;
        let reader = io::BufReader::new(file);

        debug!("trying to read save from {}", save_path.as_ref().display());
        Ok(Self {
            save: serde_json::from_reader(reader)?,
            path: save_path.into(),
        })
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
        serde_json::to_writer_pretty(writer, &self.save).map_err(|err| err.into())
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
    // Add `tag` to `self`, if it isn't already included in the labels or object tags
    pub fn push_object_tag(&mut self, tag: Tag) -> bool {
        let label = Label::from(tag.clone());
        let objects_include = self
            .objects
            .iter()
            .any(|object| object.tags.iter().any(|t| t == &tag));

        if !self.tags.labels.contains(&label) && !objects_include {
            self.tags.labels.push(label);
            info!("added {} as a component tag", tag);
            true
        } else {
            false
        }
    }

    // Remove component tags that exist as object tags
    pub fn remove_object_tags(&mut self) {
        self.tags.labels.retain(|label| {
            !self.objects.iter().any(|object| {
                object
                    .tags
                    .iter()
                    .any(|tag| &Label::from(tag.clone()) == label)
            })
        })
    }
}
