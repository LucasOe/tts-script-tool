use std::collections::HashMap;

use crate::error::Result;
use crate::tags::Tags;
use derive_more::{Deref, DerefMut, IntoIterator};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize, Serialize, Default, Clone, Debug, IntoIterator, Deref, DerefMut)]
pub struct Objects(Vec<Object>);

impl From<Vec<Object>> for Objects {
    fn from(vec: Vec<Object>) -> Self {
        Objects(vec)
    }
}

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

    /// Replace all the objects in `self` with `other`, where their guid matches.
    pub fn replace(&mut self, other: &mut [Object]) {
        for object_state in &mut self.0 {
            if let Some(object) = other.iter().find(|object| object.guid == object_state.guid) {
                *object_state = object.clone();
            };
        }
    }

    /// Searches for an object that has the same guid.
    pub fn find_object(self, guid: &String) -> Result<Object> {
        self.into_iter()
            .find(|object| &object.guid == guid)
            .ok_or("{guid} does not exist".into())
    }

    /// Filter out `HandTrigger`, `FogOfWar` and `FogOfWarTrigger` objects.
    ///
    /// For a list of object names see:
    /// https://kb.tabletopsimulator.com/custom-content/save-file-format/#object-name-list
    pub fn filter_hidden(self) -> Self {
        const HIDDEN: &[&str] = &["HandTrigger", "FogOfWar", "FogOfWarTrigger"];
        self.into_iter()
            .filter(|object| !HIDDEN.contains(&object.name.as_str()))
            .collect()
    }

    /// Construct a vec of [`serde_json::Value`] from `self`.
    /// The value only includes the `guid`, `lau_script` and `xml_ui`.
    pub fn to_values(&self) -> Vec<Value> {
        self.iter().map(|object| object.to_value()).collect()
    }
}

/// An object loaded in the current save or savestate.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Object {
    #[serde(rename = "GUID")]
    pub guid: String,
    #[serde(rename = "LuaScript", default)]
    pub lua_script: String,
    #[serde(rename = "XmlUI", default)]
    pub xml_ui: String,
    #[serde(rename = "Name", default)]
    pub name: String,
    #[serde(rename = "Nickname", default)]
    pub nickname: String,
    #[serde(rename = "Tags", default)]
    pub tags: Tags,

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

impl Object {
    /// Construct a [`serde_json::Value`] from `self`.
    /// The value only includes the `guid`, `lau_script` and `xml_ui`.
    pub fn to_value(&self) -> Value {
        serde_json::json!({
            "guid": self.guid,
            "script": self.lua_script,
            "ui": self.xml_ui,
        })
    }
}
