use crate::error::{Error, Result};
use crate::execute;
use crate::tags::Tags;
use derive_more::{Deref, DerefMut, Display, IntoIterator};
use serde::{Deserialize, Serialize};
use tts_external_api::ExternalEditorApi;

#[derive(Deserialize, Serialize, Clone, Debug, IntoIterator, Deref, DerefMut)]
pub struct Objects(Vec<Object>);

impl Objects {
    /// Get a list of objects in the current save
    pub fn request(api: &ExternalEditorApi) -> Result<Self> {
        execute!(
            api,
            r#"
                list = {{}}
                for _, obj in pairs(getAllObjects()) do
                    table.insert(list, {{
                        guid = obj.guid,
                        name = obj.name,
                        script = obj.script_code,
                    }})
                end
                return JSON.encode(list)
            "#,
        )
    }

    /// Get a list of script states from the current save
    pub fn request_script_states(api: &ExternalEditorApi) -> Result<Self> {
        let script_states = api.get_scripts()?.script_states;
        serde_json::from_value(script_states).map_err(Error::SerdeError)
    }

    /// Get [`Object`] by guid
    pub fn find(self, guid: &String) -> Option<Object> {
        self.into_iter()
            .find(|script_state| &script_state.guid == guid)
    }

    /// Get global [`Object`]
    pub fn global(self) -> Option<Object> {
        self.find(&String::from("-1"))
    }

    pub fn as_vec(self) -> Vec<Object> {
        self.0
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, Display)]
#[display(fmt = "{}", guid)]
pub struct Object {
    pub guid: String,
    pub name: Option<String>,
    pub script: Option<String>,
    pub ui: Option<String>,
}

impl Object {
    pub fn new(guid: String) -> Self {
        Self {
            guid,
            name: None,
            script: None,
            ui: None,
        }
    }

    pub fn script(&self) -> String {
        match &self.script {
            Some(script) => script.clone(),
            None => String::new(),
        }
    }

    pub fn ui(&self) -> String {
        match &self.ui {
            Some(ui) => ui.clone(),
            None => String::new(),
        }
    }

    /// Returns a list of tags for this object
    pub fn tags(&self, api: &ExternalEditorApi) -> Result<Tags> {
        execute!(
            api,
            r#"
				return JSON.encode(getObjectFromGUID("{guid}").getTags())
			"#,
            guid = self.guid
        )
    }

    /// Sets a list of tags for this object
    pub fn set_tags(&self, api: &ExternalEditorApi, tags: &Tags) -> Result<()> {
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

    /// Set the script for this object
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

    pub fn exists(&self, api: &ExternalEditorApi) -> Result<Self> {
        let objects = Objects::request(api)?;
        match objects.find(&self.guid) {
            Some(object) => Ok(object),
            None => Err(format!("{self:?} does not exist").into()),
        }
    }
}
