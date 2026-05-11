use std::collections::HashMap;

use bevy::prelude::*;

use crate::world::structures::StructureKind;

/// Stores one HUD block config shared by world cells and structures.
///
/// # Fields
/// - `sort_order`: Relative ordering value used when multiple HUD blocks are rendered together.
/// - `substance_containers`: Substance containers shown inside this HUD block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HudBlockConfig {
    pub sort_order: i32,
    pub substance_containers: Vec<SubstanceContainerConfig>,
}

/// Stores the currently supported substance kinds for HUD container output.
///
/// # Variants
/// - `Gas`: Container displays a gas mixture or gas-only summary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubstanceKind {
    Gas,
}

/// Stores the storage backend used by one HUD substance container.
///
/// # Variants
/// - `WorldCell`: Container reads substances directly from the hovered world cell.
/// - `PipeNode`: Container reads substances from a specific pipe node representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainerBacking {
    WorldCell,
    PipeNode { kind: ConfiguredPipeNodeKind },
}

/// Stores supported pipe-node kinds referenced by HUD config.
///
/// # Variants
/// - `Pipe`: Container reads from a normal pipe node.
/// - `BridgePipe`: Container reads from the internal pipe segment of a bridge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfiguredPipeNodeKind {
    Pipe,
    BridgePipe,
}

/// Stores when a substance container becomes visible in the HUD.
///
/// # Variants
/// - `SameCell`: Container appears when the hovered cell is the same cell as the backing storage.
/// - `ContainerCell`: Container appears when the hovered cell matches the backing container cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoverVisibility {
    SameCell,
    ContainerCell,
}

/// Stores one config entry for a HUD substance container.
///
/// # Fields
/// - `substance`: Substance category displayed by this container.
/// - `backing`: Runtime storage source that feeds the container data.
/// - `visible_on_hover`: Hover rule that controls when the container is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubstanceContainerConfig {
    pub substance: SubstanceKind,
    pub backing: ContainerBacking,
    pub visible_on_hover: HoverVisibility,
}

#[derive(Resource, Clone, Debug, PartialEq, Eq)]
/// Stores the HUD config for the hovered world cell block.
///
/// # Fields
/// - `label`: Block title shown for hovered world-cell data.
/// - `block`: Shared HUD block configuration used to render the cell contents.
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
