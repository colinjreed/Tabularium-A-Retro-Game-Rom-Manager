//! Settings persisted between runs (via eframe's storage).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tabularium_core::scrapers::screenscraper::Credentials;

pub const KEY: &str = "rom-manager-settings";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub library: Option<PathBuf>,
    pub recent_libraries: Vec<PathBuf>,
    pub screenscraper: Credentials,
    /// Save ScreenScraper passwords in the settings file.
    pub remember_passwords: bool,
    /// libretro fuzzy-match strictness, 0.5..1.0.
    pub match_cutoff: f64,
    pub last_card_folder: Option<PathBuf>,
    /// Show Super Famicom / Mega Drive style names instead of US ones.
    pub system_names: tabularium_core::systems::NameRegion,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            library: None,
            recent_libraries: vec![],
            screenscraper: Credentials::default(),
            remember_passwords: false,
            match_cutoff: 0.92,
            last_card_folder: None,
            system_names: Default::default(),
        }
    }
}

impl Settings {
    pub fn load(storage: Option<&dyn eframe::Storage>) -> Settings {
        storage
            .and_then(|s| eframe::get_value(s, KEY))
            .unwrap_or_default()
    }

    pub fn save(&self, storage: &mut dyn eframe::Storage) {
        let mut copy = self.clone();
        if !copy.remember_passwords {
            copy.screenscraper.password.clear();
            copy.screenscraper.dev_password.clear();
        }
        eframe::set_value(storage, KEY, &copy);
    }

    pub fn remember_library(&mut self, path: PathBuf) {
        self.recent_libraries.retain(|p| p != &path);
        self.recent_libraries.insert(0, path.clone());
        self.recent_libraries.truncate(8);
        self.library = Some(path);
    }
}
