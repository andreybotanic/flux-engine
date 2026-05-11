use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use bevy::prelude::Resource;

use crate::{
    config::{HudBlockConfig, VisualPlacementConfig, WorldCellHudConfig},
    plugins::{PluginId, SubstanceDefinition, SubstanceId},
    render::OverlayMode,
    world::{
        grid::CellMaterial,
        structures::{StructureDescriptor, StructureKind, StructureRotation},
    },
};

/// Canonical identifier of one gameplay content item registered by a plugin.
///
/// # Fields
/// Public fields of `ContentId` are part of the generated SDK reference.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentId(String);

impl ContentId {
    /// Parses and validates one content identifier.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `parse` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn parse(raw: &str) -> Result<Self, String> {
        validate_content_id(raw)?;
        Ok(Self(raw.to_string()))
    }

    /// Returns the canonical string value.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `as_str` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Describes sprite assets used by one content item.
///
/// # Fields
/// Public fields of `SpriteMetadata` are part of the generated SDK reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpriteMetadata {
    pub image_path: String,
    pub silhouette_path: Option<String>,
    pub overlay_path: Option<String>,
}

/// Describes how legacy save/runtime state still stores one content item.
///
/// # Variants
/// Public variants of `LegacyStorageDescriptor` are listed in the Rust declaration and documented by the generated SDK reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LegacyStorageDescriptor {
    WorldCellCode(u8),
    PlacedStructureKind(&'static str),
    OverlayMode(&'static str),
}

/// Describes one cell material registered by a content plugin.
///
/// # Fields
/// Public fields of `CellContentDescriptor` are part of the generated SDK reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellContentDescriptor {
    pub id: ContentId,
    pub plugin_id: PluginId,
    pub material: CellMaterial,
    pub config_file_name: &'static str,
    pub visual: VisualPlacementConfig,
    pub layer_descriptor: StructureDescriptor,
    pub sprite: SpriteMetadata,
    pub storage: LegacyStorageDescriptor,
}

/// Describes one placeable structure registered by a content plugin.
///
/// # Fields
/// Public fields of `StructureContentDescriptor` are part of the generated SDK reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructureContentDescriptor {
    pub id: ContentId,
    pub plugin_id: PluginId,
    pub kind: StructureKind,
    pub config_file_name: &'static str,
    pub visual: VisualPlacementConfig,
    pub layer_descriptors: BTreeMap<StructureRotation, StructureDescriptor>,
    pub allowed_rotations: Vec<StructureRotation>,
    pub sprite: SpriteMetadata,
    pub hud: HudBlockConfig,
    pub storage: LegacyStorageDescriptor,
}

impl StructureContentDescriptor {
    /// Returns the layer descriptor for one rotation.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `layer_descriptor` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn layer_descriptor(&self, rotation: StructureRotation) -> &StructureDescriptor {
        self.layer_descriptors
            .get(&rotation)
            .unwrap_or_else(|| panic!("missing layer descriptor for {:?}", rotation))
    }
}

/// Describes one overlay mode registered by a content plugin.
///
/// # Fields
/// Public fields of `OverlayContentDescriptor` are part of the generated SDK reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlayContentDescriptor {
    pub id: ContentId,
    pub plugin_id: PluginId,
    pub mode: OverlayMode,
    pub label: &'static str,
    pub hotkey: &'static str,
    pub storage: LegacyStorageDescriptor,
}

/// Runtime registry of content provider plugins and their registered content descriptors.
///
/// # Fields
/// Public fields of `ContentRegistry` are part of the generated SDK reference.
#[derive(Resource, Clone, Debug, Default)]
pub struct ContentRegistry {
    provider_plugins: BTreeSet<PluginId>,
    cells: BTreeMap<ContentId, CellContentDescriptor>,
    structures: BTreeMap<ContentId, StructureContentDescriptor>,
    overlays: BTreeMap<ContentId, OverlayContentDescriptor>,
    substances: BTreeMap<SubstanceId, SubstanceDefinition>,
    world_cell_hud: Option<WorldCellHudConfig>,
}

impl ContentRegistry {
    /// Adds one plugin to the set of content providers.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `register_provider_plugin` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn register_provider_plugin(&mut self, plugin_id: PluginId) {
        self.provider_plugins.insert(plugin_id);
    }

    /// Registers one cell material descriptor.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `register_cell` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn register_cell(&mut self, descriptor: CellContentDescriptor) {
        self.cells.insert(descriptor.id.clone(), descriptor);
    }

    /// Registers one structure descriptor.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `register_structure` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn register_structure(&mut self, descriptor: StructureContentDescriptor) {
        self.structures.insert(descriptor.id.clone(), descriptor);
    }

    /// Registers one overlay mode descriptor.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `register_overlay` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn register_overlay(&mut self, descriptor: OverlayContentDescriptor) {
        self.overlays.insert(descriptor.id.clone(), descriptor);
    }

    /// Registers one substance definition.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `register_substance` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn register_substance(&mut self, definition: SubstanceDefinition) {
        self.substances.insert(definition.id.clone(), definition);
    }

    /// Stores the default world-cell HUD descriptor.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `set_world_cell_hud` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn set_world_cell_hud(&mut self, descriptor: WorldCellHudConfig) {
        self.world_cell_hud = Some(descriptor);
    }

    /// Returns the registered content-provider plugin ids.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `provider_plugins` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn provider_plugins(&self) -> &BTreeSet<PluginId> {
        &self.provider_plugins
    }

    /// Returns every registered cell descriptor by stable content id.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `cells` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn cells(&self) -> &BTreeMap<ContentId, CellContentDescriptor> {
        &self.cells
    }

    /// Returns every registered structure descriptor by stable content id.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `structures` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn structures(&self) -> &BTreeMap<ContentId, StructureContentDescriptor> {
        &self.structures
    }

    /// Returns every registered overlay descriptor by stable content id.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `overlays` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn overlays(&self) -> &BTreeMap<ContentId, OverlayContentDescriptor> {
        &self.overlays
    }

    /// Returns every registered substance definition by stable substance id.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `substances` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn substances(&self) -> &BTreeMap<SubstanceId, SubstanceDefinition> {
        &self.substances
    }

    /// Returns the configured world-cell HUD descriptor.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `world_cell_hud` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn world_cell_hud(&self) -> Option<&WorldCellHudConfig> {
        self.world_cell_hud.as_ref()
    }

    /// Finds a registered cell descriptor by the legacy runtime material.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `cell_by_material` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn cell_by_material(&self, material: CellMaterial) -> Option<&CellContentDescriptor> {
        self.cells
            .values()
            .find(|descriptor| descriptor.material == material)
    }

    /// Finds a registered structure descriptor by the legacy runtime kind.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `structure_by_kind` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn structure_by_kind(&self, kind: StructureKind) -> Option<&StructureContentDescriptor> {
        self.structures
            .values()
            .find(|descriptor| descriptor.kind == kind)
    }

    /// Finds a registered overlay descriptor by the legacy runtime mode.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `overlay_by_mode` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn overlay_by_mode(&self, mode: OverlayMode) -> Option<&OverlayContentDescriptor> {
        self.overlays
            .values()
            .find(|descriptor| descriptor.mode == mode)
    }
}

fn validate_content_id(raw: &str) -> Result<(), String> {
    if raw.is_empty() {
        return Err("content id must not be empty".to_string());
    }

    let bytes = raw.as_bytes();
    let first = bytes[0];
    if !is_lower_ascii_alphanumeric(first) {
        return Err(format!(
            "content id '{}' must start with a lowercase ASCII letter or digit",
            raw
        ));
    }

    let last = *bytes.last().expect("non-empty checked above");
    if !is_lower_ascii_alphanumeric(last) {
        return Err(format!(
            "content id '{}' must end with a lowercase ASCII letter or digit",
            raw
        ));
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

        return Err(format!(
            "content id '{}' must match ^[a-z0-9]+([._-][a-z0-9]+)*$",
            raw
        ));
    }

    Ok(())
}

fn is_lower_ascii_alphanumeric(value: u8) -> bool {
    value.is_ascii_lowercase() || value.is_ascii_digit()
}
