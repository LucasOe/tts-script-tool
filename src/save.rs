use std::collections::HashMap;

use log::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::objects::Objects;
use crate::tags::Label;
use crate::Tag;

#[derive(Deserialize, Serialize, Debug)]
pub struct ComponentTags {
    pub labels: Vec<Label>,
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
    /// Add `tag` to `self`, if it isn't already included in the labels or object tags
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

    /// Remove component tags that exist as object tags
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
