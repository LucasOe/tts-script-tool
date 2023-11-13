use std::path::Path;
use std::path::PathBuf;
use std::{fs, io};

use log::debug;
use tts_external_api::ExternalEditorApi;

use ttsst::error::Result;
use ttsst::Save;

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
