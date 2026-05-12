use std::{
    fmt,
    path::{Path, PathBuf},
};

/// Strongly typed wrapper around one plugin ABI version.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PluginApiVersion(pub u32);

impl PluginApiVersion {
    /// Returns the raw ABI version value.
    pub fn value(self) -> u32 {
        self.0
    }
}

impl fmt::Display for PluginApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Canonical plugin identifier.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PluginId(String);

impl PluginId {
    /// Parses one plugin identifier.
    pub fn parse(raw: &str) -> Result<Self, String> {
        validate_stable_id(raw, "plugin id")?;
        Ok(Self(raw.to_string()))
    }

    /// Returns the canonical identifier string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PluginId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Canonical gameplay content identifier.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentId(String);

impl ContentId {
    /// Parses one content identifier.
    pub fn parse(raw: &str) -> Result<Self, String> {
        validate_stable_id(raw, "content id")?;
        Ok(Self(raw.to_string()))
    }

    /// Returns the canonical identifier string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Canonical gas-substance identifier.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubstanceId(String);

impl SubstanceId {
    /// Parses one substance identifier.
    pub fn parse(raw: &str) -> Result<Self, String> {
        validate_stable_id(raw, "substance id")?;
        Ok(Self(raw.to_string()))
    }

    /// Returns the canonical identifier string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SubstanceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Runtime entity instance identifier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityInstanceId(pub u32);

/// Stable entity-kind identifier.
pub type EntityKindId = ContentId;
/// Stable layer identifier.
pub type EntityLayerId = ContentId;
/// Stable overlay mode identifier.
pub type OverlayModeId = ContentId;
/// Cell coordinates in world space.
pub type CellPos = bevy_math::UVec2;

/// One inclusive cell rectangle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CellRect {
    pub min: CellPos,
    pub max: CellPos,
}

/// World bounds in cells.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorldBounds {
    pub width: u32,
    pub height: u32,
}

/// Engine paths visible to the plugin during creation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginPaths {
    pub plugin_root: PathBuf,
    pub config_root: PathBuf,
    pub assets_root: PathBuf,
}

impl PluginPaths {
    /// Returns the plugin root directory.
    pub fn plugin_root(&self) -> &Path {
        &self.plugin_root
    }

    /// Returns the config root directory.
    pub fn config_root(&self) -> &Path {
        &self.config_root
    }

    /// Returns the assets root directory.
    pub fn assets_root(&self) -> &Path {
        &self.assets_root
    }
}

fn validate_stable_id(raw: &str, label: &str) -> Result<(), String> {
    if raw.is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    let bytes = raw.as_bytes();
    let valid_edge = |value: u8| value.is_ascii_lowercase() || value.is_ascii_digit();
    if !valid_edge(bytes[0]) || !valid_edge(*bytes.last().expect("non-empty")) {
        return Err(format!(
            "{label} '{raw}' must start and end with a lowercase ASCII letter or digit"
        ));
    }
    let mut previous_sep = false;
    for byte in bytes {
        if valid_edge(*byte) {
            previous_sep = false;
            continue;
        }
        if matches!(*byte, b'.' | b'_' | b'-') && !previous_sep {
            previous_sep = true;
            continue;
        }
        return Err(format!(
            "{label} '{raw}' must match ^[a-z0-9]+([._-][a-z0-9]+)*$"
        ));
    }
    Ok(())
}
