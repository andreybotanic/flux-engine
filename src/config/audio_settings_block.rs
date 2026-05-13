use serde::Serialize;

const SETTINGS_FILE_NAME: &str = "settings.toml";

/// Default music volume percent used when no persisted settings exist.
pub const DEFAULT_MUSIC_VOLUME_PERCENT: u32 = 50;

#[derive(Deserialize, Serialize)]
struct SettingsToml {
    #[serde(default)]
    audio: AudioToml,
}

#[derive(Deserialize, Serialize)]
struct AudioToml {
    #[serde(default = "default_music_volume_percent")]
    music_volume: u32,
}

impl Default for AudioToml {
    fn default() -> Self {
        Self {
            music_volume: default_music_volume_percent(),
        }
    }
}

fn default_music_volume_percent() -> u32 {
    DEFAULT_MUSIC_VOLUME_PERCENT
}

fn clamp_music_volume_percent(value: u32) -> u32 {
    value.min(100)
}

/// Converts one 0..100 music volume percentage into normalized 0.0..1.0 volume.
pub fn normalize_music_volume_percent(value: u32) -> f32 {
    (clamp_music_volume_percent(value) as f32 / 100.0).clamp(0.0, 1.0)
}

/// Runtime sound settings used by UI and BGM.
///
/// `saved_music_volume_percent` mirrors the persisted value on disk, while
/// `runtime_music_volume_percent` tracks the current live slider value.
#[derive(Resource, Clone, Debug)]
pub struct AudioSettingsState {
    pub saved_music_volume_percent: u32,
    pub runtime_music_volume_percent: u32,
}

impl Default for AudioSettingsState {
    fn default() -> Self {
        Self {
            saved_music_volume_percent: DEFAULT_MUSIC_VOLUME_PERCENT,
            runtime_music_volume_percent: DEFAULT_MUSIC_VOLUME_PERCENT,
        }
    }
}

impl AudioSettingsState {
    /// Loads sound settings from the repository `config/settings.toml` file.
    ///
    /// Missing files are treated as defaults.
    pub fn load_from_default_location() -> Result<Self, String> {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let config_root = repo_root.join("config");
        Self::load_from_root(&config_root)
    }

    /// Loads sound settings from one explicit config directory.
    pub fn load_from_root(config_root: &Path) -> Result<Self, String> {
        let path = config_root.join(SETTINGS_FILE_NAME);
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)
            .map_err(|err| format!("Failed to read settings config '{}': {}", path.display(), err))?;
        let parsed = toml::from_str::<SettingsToml>(&content)
            .map_err(|err| format!("Failed to parse settings config '{}': {}", path.display(), err))?;
        let saved = clamp_music_volume_percent(parsed.audio.music_volume);
        Ok(Self {
            saved_music_volume_percent: saved,
            runtime_music_volume_percent: saved,
        })
    }

    /// Saves the current runtime slider value into `config/settings.toml`.
    pub fn save_runtime_to_default_location(&mut self) -> Result<(), String> {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let config_root = repo_root.join("config");
        self.save_runtime_to_root(&config_root)
    }

    /// Saves the current runtime slider value into one explicit config directory.
    pub fn save_runtime_to_root(&mut self, config_root: &Path) -> Result<(), String> {
        fs::create_dir_all(config_root).map_err(|err| {
            format!(
                "Failed to create settings config directory '{}': {}",
                config_root.display(),
                err
            )
        })?;
        let value = clamp_music_volume_percent(self.runtime_music_volume_percent);
        let raw = SettingsToml {
            audio: AudioToml { music_volume: value },
        };
        let text = toml::to_string_pretty(&raw).map_err(|err| {
            format!(
                "Failed to serialize settings config '{}': {}",
                config_root.join(SETTINGS_FILE_NAME).display(),
                err
            )
        })?;
        let path = config_root.join(SETTINGS_FILE_NAME);
        fs::write(&path, text)
            .map_err(|err| format!("Failed to write settings config '{}': {}", path.display(), err))?;
        self.saved_music_volume_percent = value;
        self.runtime_music_volume_percent = value;
        Ok(())
    }

    /// Updates the live runtime slider value using one clamped 0..100 percentage.
    pub fn set_runtime_music_volume_percent(&mut self, value: u32) {
        self.runtime_music_volume_percent = clamp_music_volume_percent(value);
    }

    /// Returns the normalized 0.0..1.0 volume currently applied at runtime.
    pub fn runtime_music_volume_normalized(&self) -> f32 {
        normalize_music_volume_percent(self.runtime_music_volume_percent)
    }
}

#[cfg(test)]
mod audio_settings_tests {
    use super::{normalize_music_volume_percent, AudioSettingsState};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_config_dir() -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("flux_audio_settings_{unique}"))
    }

    #[test]
    fn normalize_music_volume_percent_clamps_into_unit_range() {
        assert_eq!(normalize_music_volume_percent(0), 0.0);
        assert_eq!(normalize_music_volume_percent(50), 0.5);
        assert_eq!(normalize_music_volume_percent(100), 1.0);
        assert_eq!(normalize_music_volume_percent(250), 1.0);
    }

    #[test]
    fn load_defaults_when_settings_file_is_missing() {
        let root = temp_config_dir();
        std::fs::create_dir_all(&root).expect("create root");
        let loaded = AudioSettingsState::load_from_root(&root).expect("load defaults");
        assert_eq!(loaded.saved_music_volume_percent, 50);
        assert_eq!(loaded.runtime_music_volume_percent, 50);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn save_then_reload_preserves_volume_percent() {
        let root = temp_config_dir();
        std::fs::create_dir_all(&root).expect("create root");

        let mut state = AudioSettingsState::default();
        state.set_runtime_music_volume_percent(73);
        state.save_runtime_to_root(&root).expect("save settings");
        assert_eq!(state.saved_music_volume_percent, 73);
        assert_eq!(state.runtime_music_volume_percent, 73);

        let loaded = AudioSettingsState::load_from_root(&root).expect("reload settings");
        assert_eq!(loaded.saved_music_volume_percent, 73);
        assert_eq!(loaded.runtime_music_volume_percent, 73);

        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
