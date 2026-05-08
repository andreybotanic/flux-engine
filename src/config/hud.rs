use std::collections::HashMap;

use bevy::prelude::*;

use crate::world::structures::StructureKind;

/// Stores one HUD block config shared by world cells and structures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HudBlockConfig {
    pub sort_order: i32,
    pub substance_containers: Vec<SubstanceContainerConfig>,
}

/// Stores the currently supported substance kinds for HUD container output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubstanceKind {
    Gas,
}

/// Stores the storage backend used by one HUD substance container.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainerBacking {
    WorldCell,
    PipeNode { kind: ConfiguredPipeNodeKind },
}

/// Stores supported pipe-node kinds referenced by HUD config.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfiguredPipeNodeKind {
    Pipe,
    BridgePipe,
}

/// Stores when a substance container becomes visible in the HUD.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoverVisibility {
    SameCell,
    ContainerCell,
}

/// Stores one config entry for a HUD substance container.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubstanceContainerConfig {
    pub substance: SubstanceKind,
    pub backing: ContainerBacking,
    pub visible_on_hover: HoverVisibility,
}

#[derive(Resource, Clone, Debug, PartialEq, Eq)]
/// Stores the HUD config for the hovered world cell block.
pub struct WorldCellHudConfig {
    pub label: String,
    pub block: HudBlockConfig,
}

#[derive(Resource, Clone, Debug, PartialEq, Eq)]
/// Stores HUD configs for every built-in structure kind.
pub struct StructureHudConfigMap {
    configs: HashMap<StructureKind, HudBlockConfig>,
}

impl StructureHudConfigMap {
    /// Returns the HUD config for the requested structure kind.
    pub fn get(&self, kind: StructureKind) -> &HudBlockConfig {
        self.configs
            .get(&kind)
            .unwrap_or_else(|| panic!("missing structure HUD config for {:?}", kind))
    }

    pub(crate) fn from_entries(entries: Vec<(StructureKind, HudBlockConfig)>) -> Self {
        Self {
            configs: entries.into_iter().collect(),
        }
    }
}
