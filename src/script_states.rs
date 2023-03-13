use crate::error::{Error, Result};
use serde::Deserialize;
use tts_external_api::ExternalEditorApi;

#[derive(Deserialize, Clone, Debug)]
pub struct ScriptStates(Vec<ScriptState>);

impl ScriptStates {
    /// Get a list of ScriptStates in the current save
    pub fn new(api: &ExternalEditorApi) -> Result<Self> {
        let script_states = api.get_scripts()?.script_states;
        serde_json::from_value(script_states).map_err(Error::SerdeError)
    }

    /// Get [`ScriptState`] by guid
    pub fn get(&self, guid: String) -> Option<ScriptState> {
        self.0
            .clone()
            .into_iter()
            .find(|script_state| script_state.guid == guid)
    }

    /// Get global [`ScriptState`]
    pub fn global(&self) -> Option<ScriptState> {
        self.get(String::from("-1"))
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct ScriptState {
    pub guid: String,
    pub name: Option<String>,
    pub script: Option<String>,
    pub ui: Option<String>,
}

impl ScriptState {
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
}
