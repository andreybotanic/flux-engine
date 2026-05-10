use std::path::PathBuf;

use serde::Deserialize;

use crate::plugins::{
    diagnostics::PluginContractError,
    id::{PluginApiVersion, PluginId, PluginVersion, ENGINE_PLUGIN_API_VERSION},
    source::validate_relative_plugin_path,
};

/// Validated plugin manifest used by the engine runtime.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginManifest {
    pub id: PluginId,
    pub display_name: String,
    pub version: PluginVersion,
    pub api_version: PluginApiVersion,
    pub dll: PathBuf,
    pub configs: PathBuf,
    pub assets: PathBuf,
    pub content: bool,
    pub description: Option<String>,
}

impl PluginManifest {
    /// Parses and validates a TOML manifest string.
    pub fn from_str(value: &str) -> Result<Self, PluginContractError> {
        let raw = toml::from_str::<RawPluginManifest>(value).map_err(|error| {
            PluginContractError::Manifest(format!("failed to parse manifest.toml: {}", error))
        })?;
        Self::from_raw(raw)
    }

    /// Parses and validates UTF-8 bytes from one `manifest.toml`.
    pub fn from_bytes(value: &[u8]) -> Result<Self, PluginContractError> {
        let text = std::str::from_utf8(value).map_err(|error| {
            PluginContractError::Manifest(format!("manifest.toml is not valid UTF-8: {}", error))
        })?;
        Self::from_str(text)
    }

    fn from_raw(raw: RawPluginManifest) -> Result<Self, PluginContractError> {
        let id = PluginId::parse(&raw.id)?;
        let display_name = raw.display_name.trim();
        if display_name.is_empty() {
            return Err(PluginContractError::Manifest(
                "display_name must not be empty".to_string(),
            ));
        }

        let version = PluginVersion::parse(&raw.version)?;
        let api_version = PluginApiVersion::new(raw.api_version);
        if api_version != ENGINE_PLUGIN_API_VERSION {
            return Err(PluginContractError::Manifest(format!(
                "unsupported api_version '{}'; engine supports '{}'",
                api_version, ENGINE_PLUGIN_API_VERSION
            )));
        }

        let dll = validate_relative_plugin_path(&raw.dll)?;
        let configs = validate_relative_plugin_path(&raw.configs)?;
        let assets = validate_relative_plugin_path(&raw.assets)?;
        let description = raw
            .description
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        Ok(Self {
            id,
            display_name: display_name.to_string(),
            version,
            api_version,
            dll,
            configs,
            assets,
            content: raw.content,
            description,
        })
    }
}

#[derive(Deserialize)]
struct RawPluginManifest {
    id: String,
    display_name: String,
    version: String,
    api_version: u32,
    dll: String,
    configs: String,
    assets: String,
    content: bool,
    description: Option<String>,
}

#[cfg(test)]
mod tests {
    use crate::plugins::PluginManifest;

    #[test]
    fn plugin_contract_manifest_accepts_valid_manifest() {
        let manifest = PluginManifest::from_str(
            r#"id = "valid.plugin"
display_name = "Valid Plugin"
version = "1.2.3"
api_version = 1
dll = "bin/valid.dll"
configs = "config"
assets = "assets"
content = false
description = "test"
"#,
        )
        .expect("manifest should parse");

        assert_eq!(manifest.id.as_str(), "valid.plugin");
        assert_eq!(manifest.display_name, "Valid Plugin");
        assert_eq!(manifest.version.to_string(), "1.2.3");
    }

    #[test]
    fn plugin_contract_manifest_rejects_empty_id() {
        let error = PluginManifest::from_str(
            r#"id = ""
display_name = "Broken"
version = "1.2.3"
api_version = 1
dll = "bin/valid.dll"
configs = "config"
assets = "assets"
content = false
"#,
        )
        .expect_err("empty id must fail");

        assert!(error.to_string().contains("plugin id must not be empty"));
    }

    #[test]
    fn plugin_contract_manifest_rejects_incompatible_api_version() {
        let error = PluginManifest::from_str(
            r#"id = "invalid.api"
display_name = "Broken"
version = "1.2.3"
api_version = 999
dll = "bin/valid.dll"
configs = "config"
assets = "assets"
content = false
"#,
        )
        .expect_err("incompatible api version must fail");

        assert!(error.to_string().contains("unsupported api_version '999'"));
    }
}
