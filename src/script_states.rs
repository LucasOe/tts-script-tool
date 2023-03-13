use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct ScriptState {
    pub guid: String,
    pub name: Option<String>,
    pub script: Option<String>,
    pub ui: Option<String>,
}

impl ScriptState {
    pub fn script(self) -> String {
        self.script.unwrap_or(String::new())
    }

    pub fn ui(self) -> String {
        self.ui.unwrap_or(String::new())
    }
}
