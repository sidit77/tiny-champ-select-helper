use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use anyhow::{anyhow, ensure, Result};
use directories::ProjectDirs;
use error_tools::OptionToError;
use native_dialog::{FileDialog, MessageDialog, MessageType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub client_path: String,
    pub server_url: String,
}

impl Config {

    pub fn initialize() -> Result<Self> {
        let dirs = ProjectDirs::from("com.github", "sidit77", "tiny-champ-select-helper").err()?;
        let config_path = dirs.config_dir().join("config.json");
        let config = match Self::load(&config_path) {
            Ok(config) => {
                log::info!("Config found");
                config
            }
            Err(err) => {
                log::info!("Config loading error: {}", err);
                let config = Self::new()?;
                config.save(&config_path)?;
                config
            }
        };
        Ok(config)
    }

    fn new() -> Result<Self> {
        MessageDialog::new()
            .set_text("Please select the install directory of the League Client\nFor Example: C:/Riot Games/League of Legends")
            .set_type(MessageType::Info)
            .show_alert()?;
        let path = FileDialog::new()
            .show_open_single_dir()?
            .and_then(|path| match is_valid_lcu_path(&path) {
                true => Some(path),
                false => None
            })
            .and_then(|path|path.to_str().map(|str|str.to_string()));
        match path {
            None => {
                MessageDialog::new()
                    .set_text("Invalid directory")
                    .set_type(MessageType::Error)
                    .show_alert()?;
                Err(anyhow!("No league directory"))
            }
            Some(path) => Ok(Self {
                client_path: path.to_string(),
                server_url: "127.0.0.1:43257".to_string()
            })
        }
    }

    fn save<P:  AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        fs::create_dir_all(path.parent().err()?)?;
        fs::write(&path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    fn load<P:  AsRef<Path>>(path: P) -> Result<Self> {
        let config: Self = serde_json::from_str(&fs::read_to_string(path)?)?;
        ensure!(is_valid_lcu_path(&config.client_path));
        Ok(config)
    }

}

fn is_valid_lcu_path<P:  AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();
    path.join("Config").exists() &&
        (path.join("LeagueClient.exe").exists() || path.join("LeagueClient.app").exists())
}

