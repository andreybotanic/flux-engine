use std::fmt;

use semver::Version;

use crate::plugins::diagnostics::PluginContractError;

/// Current engine-side plugin API version.
///
pub const ENGINE_PLUGIN_API_VERSION_VALUE: u32 = 7;

/// Canonical identifier of the built-in default plugin.
///
/// # SDK Notes
/// `DEFAULT_PLUGIN_ID_VALUE` is a stable constant in the Plugin SDK reference.
pub const DEFAULT_PLUGIN_ID_VALUE: &str = "flux.default";

/// Current engine-side plugin API version wrapper.
///
/// # SDK Notes
/// `ENGINE_PLUGIN_API_VERSION` keeps the current ABI version in the strongly typed wrapper used by manifests and validation code.
pub const ENGINE_PLUGIN_API_VERSION: PluginApiVersion =
    PluginApiVersion(ENGINE_PLUGIN_API_VERSION_VALUE);

/// Stable plugin API version wrapper.
///
/// # Fields
/// - `0`: Raw engine-side plugin API version number wrapped as a dedicated SDK type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PluginApiVersion(u32);

impl PluginApiVersion {
    /// Creates a plugin API version wrapper.
    ///
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    /// Returns the raw API version number.
    ///
    pub fn value(self) -> u32 {
        self.0
    }
}

impl fmt::Display for PluginApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Canonical plugin identifier used across manifests and registries.
///
/// # Fields
/// - `0`: Canonical plugin identifier string stored after validation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PluginId(String);

impl PluginId {
    /// Parses and validates one plugin identifier.
    ///
    pub fn parse(raw: &str) -> Result<Self, PluginContractError> {
        if raw.is_empty() {
            return Err(PluginContractError::Manifest(
                "plugin id must not be empty".to_string(),
            ));
        }

        let bytes = raw.as_bytes();
        let first = bytes[0];
        if !is_lower_ascii_alphanumeric(first) {
            return Err(PluginContractError::Manifest(format!(
                "plugin id '{}' must start with a lowercase ASCII letter or digit",
                raw
            )));
        }

        let last = *bytes.last().expect("non-empty checked above");
        if !is_lower_ascii_alphanumeric(last) {
            return Err(PluginContractError::Manifest(format!(
                "plugin id '{}' must end with a lowercase ASCII letter or digit",
                raw
            )));
        }

        let mut previous_was_separator = false;
        for byte in bytes {
            if is_lower_ascii_alphanumeric(*byte) {
                previous_was_separator = false;
                continue;
            }

            if matches!(*byte, b'.' | b'_' | b'-') && !previous_was_separator {
                previous_was_separator = true;
                continue;
            }

            return Err(PluginContractError::Manifest(format!(
                "plugin id '{}' must match ^[a-z0-9]+([._-][a-z0-9]+)*$",
                raw
            )));
        }

        Ok(Self(raw.to_string()))
    }

    /// Returns the canonical string value.
    ///
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the canonical built-in default plugin identifier.
    ///
    pub fn default_plugin() -> Self {
        Self::parse(DEFAULT_PLUGIN_ID_VALUE).expect("default plugin id must stay valid")
    }
}

impl fmt::Display for PluginId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Semantic plugin version declared in the manifest.
///
/// # Fields
/// - `0`: Parsed semantic version value preserved from the plugin manifest.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PluginVersion(Version);

impl PluginVersion {
    /// Parses a semantic version string.
    ///
    pub fn parse(raw: &str) -> Result<Self, PluginContractError> {
        Version::parse(raw).map(Self).map_err(|error| {
            PluginContractError::Manifest(format!(
                "plugin version '{}' is not valid semver: {}",
                raw, error
            ))
        })
    }

    /// Returns the underlying `semver::Version`.
    ///
    pub fn as_semver(&self) -> &Version {
        &self.0
    }
}

impl fmt::Display for PluginVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn is_lower_ascii_alphanumeric(value: u8) -> bool {
    value.is_ascii_lowercase() || value.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use crate::plugins::PluginId;

    #[test]
    fn plugin_contract_plugin_id_rejects_uppercase_and_spaces() {
        assert!(PluginId::parse("Bad.Plugin").is_err());
        assert!(PluginId::parse("bad plugin").is_err());
    }
}
