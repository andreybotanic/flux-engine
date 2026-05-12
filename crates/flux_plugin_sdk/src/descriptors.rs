use serde::{de::DeserializeOwned, Serialize};
use smallvec::SmallVec;

use crate::{CellPos, ContentId, EntityInstanceId, EntityKindId, OverlayModeId, PluginId, SubstanceId};

/// One plugin-owned gas substance.
#[derive(Clone, Debug, PartialEq)]
pub struct SubstanceDescriptor {
    pub id: SubstanceId,
    pub label: String,
    pub alias: String,
    pub molecular_mass: f32,
    pub color: [f32; 3],
}

/// One plugin-owned placeable entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntityDescriptor {
    pub id: EntityKindId,
    pub label: String,
    pub icon_path: String,
    pub silhouette_path: Option<String>,
}

/// One plugin-owned editor tool.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolDescriptor {
    pub id: ContentId,
    pub label: String,
    pub icon_path: String,
    pub silhouette_path: Option<String>,
}

/// One plugin-owned overlay mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlayDescriptor {
    pub id: ContentId,
    pub label: String,
    pub hotkey: Option<String>,
    pub render_policy: OverlayRenderPolicy,
}

/// One plugin-owned save chunk schema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveChunkDescriptor {
    pub id: ContentId,
    pub version: u32,
}

/// One plugin-owned save chunk payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveChunk {
    pub plugin_id: PluginId,
    pub chunk_id: ContentId,
    pub version: u32,
    pub bytes: Vec<u8>,
}

impl SaveChunk {
    /// Deserializes this chunk as JSON.
    pub fn read_json<T: DeserializeOwned>(&self) -> Result<T, crate::PluginError> {
        serde_json::from_slice(&self.bytes)
            .map_err(|error| crate::PluginError::message(format!("invalid JSON chunk: {error}")))
    }

    /// Builds a JSON save chunk.
    pub fn from_json<T: Serialize>(
        plugin_id: PluginId,
        chunk_id: ContentId,
        version: u32,
        value: &T,
    ) -> Result<Self, crate::PluginError> {
        let bytes = serde_json::to_vec(value)
            .map_err(|error| crate::PluginError::message(format!("failed to encode JSON: {error}")))?;
        Ok(Self {
            plugin_id,
            chunk_id,
            version,
            bytes,
        })
    }
}

/// One HUD block appended by a plugin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HudBlock {
    pub id: ContentId,
    pub title: String,
    pub lines: Vec<String>,
    pub sort_order: i32,
}

/// Overlay render ownership mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayRenderPolicy {
    CoreDefault,
    PluginControlled,
}

/// One RGBA8 overlay frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlayFrame {
    pub overlay_id: OverlayModeId,
    pub width: u32,
    pub height: u32,
    pub rgba8: Vec<u8>,
}

/// One entity placement request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntityPlacement {
    pub origin: CellPos,
    pub rotation: Rotation,
}

/// One entity placement request in a batch call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntitySpawnRequest {
    pub kind: EntityKindId,
    pub placement: EntityPlacement,
}

/// One entity-rotation value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rotation {
    #[default]
    Deg0,
    Deg90,
    Deg180,
    Deg270,
}

/// One placement validation result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlacementCheck {
    Allowed,
    Occupied,
    Blocked,
    OutOfBounds,
    Unsupported,
}

/// Filter used by entity search and clear operations.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EntityFilter {
    pub kinds: SmallVec<[EntityKindId; 4]>,
}

/// Minimal entity snapshot returned by read APIs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntitySnapshot {
    pub id: EntityInstanceId,
    pub kind: EntityKindId,
    pub origin: CellPos,
    pub rotation: Rotation,
    pub enabled: bool,
    pub label: Option<String>,
}

/// Minimal mutable entity-state patch.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EntityStatePatch {
    pub enabled: Option<bool>,
    pub label: Option<String>,
}

/// One world-cell snapshot returned by `WorldApi`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CellSnapshot {
    pub solid_entity: Option<EntityKindId>,
    pub entity_ids: SmallVec<[EntityInstanceId; 4]>,
}

/// One gas mixture entry.
#[derive(Clone, Debug, PartialEq)]
pub struct GasPortion {
    pub substance: SubstanceId,
    pub amount: u32,
}

/// One gas mixture view for a cell.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GasMixture {
    pub portions: Vec<GasPortion>,
}

/// Read-only substance registry view.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SubstanceRegistryView {
    pub substance_ids: Vec<SubstanceId>,
}

/// Current simulation speed multiplier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SimulationSpeed {
    #[default]
    X1,
    X2,
    X5,
}

/// Snapshot of input modifiers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

/// One editor mouse button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u16),
}

/// Extra overlay runtime state placeholder.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OverlayState {
    pub active_overlay: Option<OverlayModeId>,
    pub requested_overlay: Option<OverlayModeId>,
}
