#![allow(dead_code)]

use crate::error::{Error, Result};
use crate::execute;
use derive_more::{Display, Index, IntoIterator};
use inquire::Select;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tts_external_api::ExternalEditorApi;

#[derive(Deserialize, Serialize, Clone, Debug, IntoIterator)]
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
    pub fn script_states(api: &ExternalEditorApi) -> Result<Self> {
        let script_states = api.get_scripts()?.script_states;
        serde_json::from_value(script_states).map_err(Error::SerdeError)
    }

    /// Get [`Object`] by guid
    pub fn get(&self, guid: &String) -> Option<Object> {
        self.0
            .clone()
            .into_iter()
            .find(|script_state| &script_state.guid == guid)
    }

    /// Get global [`Object`]
    pub fn global(&self) -> Option<Object> {
        self.get(&String::from("-1"))
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
        match objects.get(&self.guid) {
            Some(object) => Ok(object),
            None => Err(format!("{self:?} does not exist").into()),
        }
    }

    pub fn select(api: &ExternalEditorApi) -> Result<Self> {
        let objects = Objects::request(api)?.0;
        Select::new("Select the object to attach the script to:", objects)
            .prompt()
            .map_err(Error::InquireError)
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, IntoIterator, Index)]
pub struct Tags(Vec<Tag>);

impl Tags {
    /// Get a list of tags for an object
    pub fn request(api: &ExternalEditorApi, guid: &str) -> Result<Self> {
        execute!(
            api,
            r#"
				return JSON.encode(getObjectFromGUID("{guid}").getTags())
			"#,
        )
    }

    pub fn add(&mut self, tag: Tag) {
        self.0.push(tag);
    }

    pub fn to_json_string(&self) -> Result<String> {
        serde_json::to_string(&self.0).map_err(Error::SerdeError)
    }

    /// Tags that follow the "scripts/<File>.ttslua" naming convention are valid
    pub fn filter_valid(&self) -> Self {
        Self(
            self.clone()
                .into_iter()
                .filter(|tag| tag.is_valid())
                .collect::<Vec<Tag>>(),
        )
    }

    /// Tags that don't follow the "scripts/<File>.ttslua" naming convention are invalid
    pub fn filter_invalid(&self) -> Self {
        Self(
            self.clone()
                .into_iter()
                .filter(|tag| !tag.is_valid())
                .collect::<Vec<Tag>>(),
        )
    }

    /// Get a valid tag
    pub fn valid(&self) -> Result<Option<Tag>> {
        let valid = self.filter_valid();
        match valid.0.len() {
            1 => Ok(valid.0.get(0).cloned()),
            0 => Ok(None),
            _ => Err("{guid} has multiple valid script tags: {tags:?}".into()),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Tag(String);

impl Tag {
    /// Create a new tag using `scripts/{file_name}` as a name
    pub fn from(path: &Path) -> Self {
        let file_name = path.file_name().unwrap().to_str().unwrap();
        Self(format!("scripts/{file_name}"))
    }

    /// Tags that follow the "scripts/<File>.ttslua" naming convention are valid
    pub fn is_valid(&self) -> bool {
        let exprs = regex::Regex::new(r"^(scripts/)[\d\w]+(\.lua|\.ttslua)$").unwrap();
        exprs.is_match(&self.0)
    }

    pub fn read_file(&self, path: &Path) -> Result<String> {
        let file_name = Path::new(&self.0).file_name().unwrap();
        let file_path = String::from(path.join(file_name).to_string_lossy());
        std::fs::read_to_string(file_path).map_err(Error::Io)
    }
}
