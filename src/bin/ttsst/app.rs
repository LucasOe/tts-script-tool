use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use colored::Colorize;
use derive_more::Display;
use itertools::Itertools;
use log::*;
use path_slash::PathExt;
use tts_external_api::ExternalEditorApi as Api;
use ttsst::save::SaveFile;
use ttsst::{Object, Objects, Save, Tag};

use crate::{Guids, ReloadArgs};

pub enum Mode {
    Attach,
    Detach,
}

impl Mode {
    pub fn msg(&self) -> &str {
        match self {
            Mode::Attach => "Select the object to attach the script or ui element to:",
            Mode::Detach => "Select the object to detach the script and ui element from:",
        }
    }
}

pub trait SaveFileExt {
    /// Attaches the script to an object by adding the script tag and the script,
    /// and then reloads the save.
    fn attach<P: AsRef<Path>>(&mut self, api: &Api, path: P, guids: Guids) -> Result<()>;

    // Detaches a script and removes all valid tags from an object.
    fn detach(&mut self, api: &Api, guids: Guids) -> Result<()>;

    /// Updates the scripts for all objects that use a script from `path`,
    /// and then reloads the save.
    fn reload<P: AsRef<Path>>(&mut self, api: &Api, paths: &[P], args: ReloadArgs) -> Result<()>
    where
        P: Clone;

    /// Backup current save as file
    fn backup<P: AsRef<Path>>(&self, path: P) -> Result<()>;

    /// Overwrite the save file and reload the current save,
    /// the same way it get reloaded when pressing “Save & Play” within the in-game editor.
    fn update_save(&mut self, api: &Api) -> Result<()>;
}

pub trait SaveExt {
    /// Set the lua script of the save to either `Global.lua` or `Global.ttslua`, if one of them exists in the `path` directory.
    /// Set the xml ui of the save to `Global.xml`, if it exists in the `path` directory.
    ///
    /// If the file is empty, this function will use a placeholder text to avoid writing an empty string.
    /// See [`Save::write`].
    fn update_global_files<P: AsRef<Path>>(&mut self, paths: &[P]) -> Result<()>;
}

impl SaveFileExt for SaveFile {
    fn attach<P: AsRef<Path>>(&mut self, api: &Api, path: P, guids: Guids) -> Result<()> {
        let mut objects = self.save.objects.get_objects(guids, Mode::Attach)?;

        let tag = Tag::try_from(path.as_ref())?;
        let file = read_file(path)?;
        for object in objects.iter_mut() {
            // Add lua tag to objects
            if tag.is_lua() {
                object.tags.retain(|tag| !tag.is_lua());
                object.tags.push(tag.clone());
                object.lua_script = file.clone();
                info!("attached script to {object}");
            }
            // Add xml tag to objects
            if tag.is_xml() {
                object.tags.retain(|tag| !tag.is_xml());
                object.tags.push(tag.clone());
                object.xml_ui = file.clone();
                info!("attached ui element to {object}");
            }
        }

        // Add objects to a new save state
        self.save.objects.replace(&mut objects);

        self.update_save(api)?;
        Ok(())
    }

    fn detach(&mut self, api: &Api, guids: Guids) -> Result<()> {
        let mut objects = self.save.objects.get_objects(guids, Mode::Detach)?;

        // Remove tags and script from objects
        for object in objects.iter_mut() {
            object.tags.retain(|tag| !tag.is_valid());
            object.lua_script = String::new();
        }

        // Add objects to a new save state
        self.save.objects.replace(&mut objects);

        self.update_save(api)?;
        Ok(())
    }

    fn reload<P: AsRef<Path> + Clone>(
        &mut self,
        api: &Api,
        paths: &[P],
        args: ReloadArgs,
    ) -> Result<()> {
        let mut has_changed = false;
        for path in &paths.reduce::<Vec<_>>() {
            // Reload objects
            if let Some(guid) = &args.guid {
                let object = self.save.objects.find_object_mut(guid)?;
                has_changed |= object.reload_object(path)?;
            } else {
                for object in self.save.objects.iter_mut() {
                    has_changed |= object.reload_object(path)?;
                }
            }

            // TODO: The watch thread does not call this function anymore
            // Add the paths as a component tag, so that in watch mode reloaded paths
            // will show up as tags.
            if let Ok(tag) = Tag::try_from(path.as_ref()) {
                has_changed |= self.save.push_object_tag(tag);
            }
        }

        if has_changed {
            self.save.update_global_files(paths)?;
            self.update_save(api)?;
        }

        Ok(())
    }

    fn backup<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        fs::copy(&self.path, &path)?;

        // Print information about the file
        let save_name = Path::new(&self.path).file_name().unwrap().to_str().unwrap();
        let path_display = path.as_ref().to_slash_lossy();
        #[rustfmt::skip]
        info!("save '{}' as '{}'", save_name.yellow(), path_display.yellow());

        Ok(())
    }

    fn update_save(&mut self, api: &Api) -> Result<()> {
        // Warning if tag an lua script or xml ui are mismatched
        for object in self.save.objects.iter() {
            if let (None, false) = (object.valid_lua()?, object.lua_script.is_empty()) {
                warn!("{} has a lua script but no valid lua tag", object);
                #[rustfmt::skip]
                warn!("If you manually removed the tag, use the detach command to remove the lua script");
            }
            if let (None, false) = (object.valid_xml()?, object.xml_ui.is_empty()) {
                warn!("{} has a xml ui but no valid xml tag", object);
                #[rustfmt::skip]
                warn!("If you manually removed the tag, use the detach command to remove the xml ui");
            }
        }

        // Remove component tags, if they exist as object tags
        self.save.remove_object_tags();

        // Overwrite the save file with the modified objects
        self.write(api)?;

        // Add global lua_script and xml_ui to save
        let mut objects = self.save.objects.to_values();
        objects.push(serde_json::json!({
            "guid": "-1",
            "script": self.save.lua_script,
            "ui": self.save.xml_ui,
        }));

        // Reload save
        api.reload(serde_json::json!(objects))?;
        info!("reloading {}", self.save.name.blue());
        Ok(())
    }
}

impl SaveExt for Save {
    fn update_global_files<P: AsRef<Path>>(&mut self, paths: &[P]) -> Result<()> {
        const GLOBAL_LUA: &[&str] = &["Global.lua", "Global.ttslua"];
        const GLOBAL_XML: &[&str] = &["Global.xml"];

        // Filter out duplicates
        let unique_paths = paths
            .iter()
            .unique_by(|path| path.as_ref().to_path_buf())
            .collect_vec();

        if let Some(path) = unique_paths.get_global_path(GLOBAL_LUA)? {
            let file = read_file(&path)?;
            let lua_script = match file.is_empty() {
                #[rustfmt::skip]
                true => "--[[ Lua code. See documentation: https://api.tabletopsimulator.com/ --]]".to_string(),
                false => file,
            };
            if self.lua_script != lua_script {
                #[rustfmt::skip]
                info!("updated {} using '{}'", "Global Lua".yellow(), path.to_slash_lossy().yellow());
                self.lua_script = lua_script;
            };
        };

        // Update xml_ui
        if let Some(path) = unique_paths.get_global_path(GLOBAL_XML)? {
            let file: String = read_file(&path)?;
            let xml_ui = match file.is_empty() {
                #[rustfmt::skip]
                true => "<!-- Xml UI. See documentation: https://api.tabletopsimulator.com/ui/introUI/ -->".to_string(),
                false => file,
            };
            if self.xml_ui != xml_ui {
                #[rustfmt::skip]
                info!("updated {} using '{}'", "Global UI".yellow(), path.to_slash_lossy().yellow());
                self.xml_ui = xml_ui;
            };
        };

        Ok(())
    }
}

trait ObjectExt {
    /// Reload the lua script and xml ui of an `object`, if its tag matches the `path`
    fn reload_object<P: AsRef<Path>>(&mut self, path: P) -> Result<bool>;
}

impl ObjectExt for Object {
    fn reload_object<P: AsRef<Path>>(&mut self, path: P) -> Result<bool> {
        // Update lua scripts if the path is a lua file
        let lua_change = match self.valid_lua()? {
            Some(tag) if tag.starts_with(&path) => {
                let file = read_file(tag.path()?)?;
                if self.lua_script != file {
                    self.lua_script = file;
                    info!("updated {self}");
                    true
                } else {
                    false
                }
            }
            // Remove lua script if the objects has no valid tag
            None if !self.lua_script.is_empty() => {
                self.lua_script = "".to_string();
                info!("removed lua script from {}", self);
                true
            }
            _ => false,
        };
        // Update xml ui if the path is a xml file
        let xml_change = match self.valid_xml()? {
            Some(tag) if tag.starts_with(&path) => {
                let file = read_file(tag.path()?)?;
                if self.xml_ui != file {
                    self.xml_ui = read_file(tag.path()?)?;
                    info!("updated {self}");
                    true
                } else {
                    false
                }
            }
            // Remove xml ui if the objects has no valid tag
            None if !self.xml_ui.is_empty() => {
                self.xml_ui = "".to_string();
                info!("removed xml ui from {}", self);
                true
            }
            _ => false,
        };

        Ok(lua_change || xml_change)
    }
}

trait ObjectsExt {
    /// If no guids are provided show a selection of objects in the current savestate.
    /// Otherwise ensure that the guids provided exist.
    fn get_objects(&self, guids: Guids, mode: Mode) -> Result<Objects>;

    /// Shows a multi selection prompt of objects loaded in the current save
    fn select_objects(&self, message: &str, show_all: bool) -> Result<Objects>;
}

impl ObjectsExt for Objects {
    fn get_objects(&self, guids: Guids, mode: Mode) -> Result<Objects> {
        match guids.guids {
            Some(guids) => self.find_objects(&guids).map_err(|err| err.into()),
            None => self.select_objects(mode.msg(), guids.all),
        }
    }

    fn select_objects(&self, message: &str, show_all: bool) -> Result<Objects> {
        let objects = match show_all {
            true => self.clone(),
            false => self.clone().filter_hidden(),
        };

        match inquire::MultiSelect::new(message, objects.into_inner()).prompt() {
            Ok(obj) => Ok(obj.into()),
            Err(err) => Err(err.into()),
        }
    }
}

trait PathsExt {
    /// Returns a path to a global script, by joining `paths` and `files`.
    fn get_global_path<T: AsRef<str>>(&self, files: &[T]) -> Result<Option<PathBuf>>;

    /// Shows a multi selection prompt of `paths`
    fn inquire_select(&self) -> Result<PathBuf>;
}

impl<P: AsRef<Path>> PathsExt for [P] {
    fn get_global_path<T: AsRef<str>>(&self, files: &[T]) -> Result<Option<PathBuf>> {
        // Returns a list of joined `paths` and `files` that exist
        let joined_paths = self
            .iter()
            .flat_map(|path| {
                files
                    .iter()
                    .filter_map(|file| {
                        let path = path.as_ref();
                        let file = file.as_ref();
                        match path.is_dir() {
                            // If path is a dir, join `file`
                            true => Some(path.join(file)),
                            // If path ends with `file`, it is a global file
                            false if path.file_name() == Some(OsStr::new(file)) => {
                                Some(path.to_path_buf())
                            }
                            // if path is a file that doesn't end with `file`, ignore it
                            false => None,
                        }
                    })
                    .filter(|path| path.exists())
                    .collect_vec()
            })
            .collect_vec();

        match joined_paths.len() {
            0 | 1 => Ok(joined_paths.get(0).map(ToOwned::to_owned)),
            _ => joined_paths.inquire_select().map(Option::Some),
        }
    }

    fn inquire_select(&self) -> Result<PathBuf> {
        #[derive(Display)]
        #[display(fmt = "'{}'", "self.0.as_ref().to_slash_lossy().yellow()")]
        struct DisplayPath<P: AsRef<Path>>(P);

        // Wrap `paths` in `DisplayPath` so they can be displayed by the inquire prompt
        let display_paths = self.iter().map(DisplayPath).collect_vec();

        match inquire::Select::new("Select a Global file to use:", display_paths).prompt() {
            Ok(path) => Ok(path.0.as_ref().to_path_buf()),
            Err(err) => Err(err.into()),
        }
    }
}

trait Reduce<P> {
    /// Filters and deduplicates the collection of paths, returning a new collection.
    ///
    /// This method removes duplicate paths based on their logical content and ensures that
    /// subfolders are not included if a parent folder is present in the collection.
    fn reduce<T: FromIterator<P>>(&self) -> T;
}

impl<U: AsRef<[P]>, P: AsRef<Path> + Clone> Reduce<P> for U {
    fn reduce<T: FromIterator<P>>(&self) -> T {
        self.as_ref()
            .iter()
            .unique_by(|path| path.as_ref().to_path_buf())
            .filter(|&this| {
                !self.as_ref().iter().any(|other| {
                    let paths = (this.as_ref(), other.as_ref());
                    paths.0 != paths.1 && paths.0.starts_with(paths.1)
                })
            })
            .cloned()
            .collect()
    }
}

/// Reads a file from the path and replaces every occurrence of `\t` with spaces.
fn read_file<P: AsRef<Path>>(path: P) -> Result<String> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(content.replace('\t', "    ")),
        Err(err) => Err(err.into()),
    }
}
