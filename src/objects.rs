use std::collections::HashMap;

use crate::error::Result;
use crate::tags::Tags;
use derive_more::{Deref, DerefMut, IntoIterator};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize, Serialize, Default, Clone, Debug, IntoIterator, Deref, DerefMut)]
pub struct Objects(Vec<Object>);

impl FromIterator<Object> for Objects {
    fn from_iter<I: IntoIterator<Item = Object>>(iter: I) -> Self {
        Objects(iter.into_iter().collect::<Vec<Object>>())
    }
}

impl Objects {
    /// Consumes `Objects`, returning the wrapped value.
    pub fn into_inner(self) -> Vec<Object> {
        self.0
    }

    /// Adds the objects to the existing objects in this `Save`.
    /// Objects with the same guid will be replaced.
    pub fn add_objects(&mut self, objects: &[Object]) -> Result<&Self> {
        for object_state in &mut self.0 {
            if let Some(object) = objects.iter().find(|object| object == &object_state) {
                *object_state = object.clone();
            };
        }

        Ok(self) // Return Self for method chaining
    }

    /// Searches for an object that has the same guid
    pub fn find_object(self, guid: &String) -> Result<Object> {
        self.into_iter()
            .find(|object| object.has_guid(guid))
            .ok_or("{guid} does not exist".into())
    }

    /// Filter out `HandTrigger`, `FogOfWar` and `FogOfWarTrigger` objects
    ///
    /// For a list of object names see:
    /// https://kb.tabletopsimulator.com/custom-content/save-file-format/#object-name-list
    pub fn filter_hidden(self) -> Self {
        const HIDDEN: &[&str] = &["HandTrigger", "FogOfWar", "FogOfWarTrigger"];

        self.into_iter()
            .filter(|object| !HIDDEN.contains(&object.name.as_str()))
            .collect()
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
