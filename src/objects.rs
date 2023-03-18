use crate::error::{Error, Result};
use crate::tags::Tags;
use crate::{execute, JsonObject};
use derive_more::{Deref, DerefMut, Display, IntoIterator};
use serde::{Deserialize, Serialize};
use tts_external_api::ExternalEditorApi;

/// A list of Objects loaded in the current save or savestate.
/// If constructed using the [`Objects::request_script_states()`] function, the global object will be included.
#[derive(Deserialize, Serialize, Clone, Debug, IntoIterator, Deref, DerefMut)]
pub struct Objects(Vec<Object>);

impl JsonObject for Objects {}

impl Objects {
    /// Returns a list of objects loaded in the current save. Only object that are interactable will be included.
    /// Objects will be constructed using their current state, which can differ from the saved data on the object.
    pub fn request(api: &ExternalEditorApi) -> Result<Self> {
        execute!(
            api,
            r#"
                list = {{}}
                for _, obj in pairs(getAllObjects()) do
                    table.insert(list, {{
                        guid = obj.guid,
                        name = obj.getName() ~= "" and obj.getName() or obj.name,
                        script = obj.script_code,
                    }})
                end
                return JSON.encode(list)
            "#,
        )
    }

    /// Returns a list of objects in the current savestate. This includes the global object.
    /// Objects will be constructed using their state in the loaded save, which can differ from their current state.
    pub fn request_script_states(api: &ExternalEditorApi) -> Result<Self> {
        let script_states = api.get_scripts()?.script_states;
        serde_json::from_value(script_states).map_err(Error::SerdeError)
    }

    /// Consumes `Objects`, returning the wrapped value.
    pub fn into_inner(self) -> Vec<Object> {
        self.0
    }

    /// Find an [`Object`] inside the list using its guid.
    pub fn find(self, guid: &String) -> Option<Object> {
        self.into_iter()
            .find(|script_state| &script_state.guid == guid)
    }

    /// Get the global script state from a list of objects.
    /// The global object will only be included in the list if it has been constructed using [`Self::request_script_states()`].
    pub fn global(self) -> Option<Object> {
        self.find(&String::from("-1"))
    }
}

/// An object loaded in the current save or savestate.
#[derive(Deserialize, Serialize, Clone, Debug, Display)]
#[display(fmt = "'{}' ({})", guid, "name.clone().unwrap_or_default()")]
pub struct Object {
    pub guid: String,
    pub name: Option<String>,
    pub script: Option<String>,
    pub ui: Option<String>,
}

impl JsonObject for Object {}

impl Object {
    /// Returns a list of [`Tags`] in the current save for this object.
    pub fn tags(&self, api: &ExternalEditorApi) -> Result<Tags> {
        execute!(
            api,
            r#"
                return JSON.encode(getObjectFromGUID("{guid}").getTags())
            "#,
            guid = self.guid
        )
    }

    /// Sets a list of [`Tags`] in the current save for this object.
    pub fn set_tags(&self, api: &ExternalEditorApi, tags: &Tags) -> Result<()> {
        println!("{}", tags.to_json_string()?.escape_default());
        execute!(
            api,
            r#"
                tags = JSON.decode("{tags}")
                getObjectFromGUID("{guid}").setTags(tags)
            "#,
            guid = self.guid,
            tags = tags.to_json_string()?.escape_default()
        )
    }

    /// Sets the script for this object in the current save.
    pub fn set_script(&self, api: &ExternalEditorApi, script: String) -> Result<()> {
        execute!(
            api,
            r#"
                getObjectFromGUID("{guid}").setLuaScript("{script}")
            "#,
            guid = self.guid,
            script = script.escape_default()
        )
    }

    /// Returns [`Self`] if the object exists in the current save.
    /// If the object does not exist, this function returns an [`Error::Msg`].
    pub fn exists(&self, api: &ExternalEditorApi) -> Result<Self> {
        let objects = Objects::request(api)?;
        match objects.find(&self.guid) {
            Some(object) => Ok(object),
            None => Err(format!("{self} does not exist").into()),
        }
    }
}
