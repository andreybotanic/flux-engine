use bevy::prelude::*;

use crate::{
    plugins::ContentId,
    world::structures::{PlacedStructureId, StructureKind},
};

/// Plugin-visible event category.
///
/// # Variants
/// Public variants of `PluginEventKind` are listed in the Rust declaration and documented by the generated SDK reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PluginEventKind {
    /// Fired after a new world is created.
    ///
    /// # SDK Example
    /// ```rust
    /// if event.event_kind == 0 {
    ///     // Initialize plugin state for a fresh world.
    /// }
    /// ```
    WorldCreated,
    /// Fired after a save slot has been loaded.
    ///
    /// # SDK Example
    /// ```rust
    /// if event.event_kind == 1 {
    ///     // Read plugin save chunks through the runtime host.
    /// }
    /// ```
    WorldLoaded,
    /// Fired synchronously before the current world is saved.
    ///
    /// # SDK Example
    /// ```rust
    /// if event.event_kind == 2 {
    ///     // Persist plugin-owned state with write_save_chunk.
    /// }
    /// ```
    WorldBeforeSave,
    /// Fired after a save operation finishes.
    ///
    /// # SDK Example
    /// ```rust
    /// if event.event_kind == 3 {
    ///     // Clear transient save status after a successful save pass.
    /// }
    /// ```
    WorldAfterSave,
    /// Fired before the current world is unloaded.
    ///
    /// # SDK Example
    /// ```rust
    /// if event.event_kind == 4 {
    ///     // Drop world-scoped plugin caches.
    /// }
    /// ```
    WorldUnloaded,
    /// Fired before the core free-gas simulation step.
    ///
    /// # SDK Example
    /// ```rust
    /// if event.event_kind == 5 {
    ///     // Inject gas before the simulation step with add_gas.
    /// }
    /// ```
    SimulationPreCellGasStep,
    /// Fired after the core free-gas simulation step.
    ///
    /// # SDK Example
    /// ```rust
    /// if event.event_kind == 6 {
    ///     // Observe post-step state or enqueue derived effects.
    /// }
    /// ```
    SimulationPostCellGasStep,
    /// Fired when the simulation pause state changes.
    ///
    /// # SDK Example
    /// ```rust
    /// if event.event_kind == 7 {
    ///     // Refresh plugin UI state that depends on pause/resume.
    /// }
    /// ```
    SimulationPausedChanged,
    /// Fired after a structure is placed.
    ///
    /// # SDK Example
    /// ```rust
    /// if event.event_kind == 8 {
    ///     // Update plugin indexes that track structures.
    /// }
    /// ```
    StructurePlaced,
    /// Fired after a structure is removed.
    ///
    /// # SDK Example
    /// ```rust
    /// if event.event_kind == 9 {
    ///     // Remove plugin metadata tied to the deleted structure.
    /// }
    /// ```
    StructureRemoved,
    /// Fired when the active editor tool changes.
    ///
    /// # SDK Example
    /// ```rust
    /// if event.event_kind == 10 {
    ///     // Read event.active_tool_id from the ABI payload.
    /// }
    /// ```
    ToolSelected,
    /// Fired when a mouse button is pressed over a world cell.
    ///
    /// # SDK Example
    /// ```rust
    /// if event.event_kind == 11 && event.has_cell != 0 {
    ///     // Use event.cell_x and event.cell_y as the target cell.
    /// }
    /// ```
    MouseDownCell,
    /// Fired when the cursor moves over world cells.
    ///
    /// # SDK Example
    /// ```rust
    /// if event.event_kind == 12 && event.has_cell != 0 {
    ///     // Continue a drag operation across cells.
    /// }
    /// ```
    MouseMoveCell,
    /// Fired when a mouse button is released over a world cell.
    ///
    /// # SDK Example
    /// ```rust
    /// if event.event_kind == 13 {
    ///     // Finish a cell drag operation.
    /// }
    /// ```
    MouseUpCell,
    /// Fired when the cursor enters a world cell.
    ///
    /// # SDK Example
    /// ```rust
    /// if event.event_kind == 14 {
    ///     // Start hover-specific plugin state.
    /// }
    /// ```
    MouseEnterCell,
    /// Fired when the cursor leaves a world cell.
    ///
    /// # SDK Example
    /// ```rust
    /// if event.event_kind == 15 {
    ///     // Clear hover-specific plugin state.
    /// }
    /// ```
    MouseLeaveCell,
    /// Fired when a key is pressed.
    ///
    /// # SDK Example
    /// ```rust
    /// if event.event_kind == 16 {
    ///     // Read event.key from the ABI payload.
    /// }
    /// ```
    KeyPressed,
    /// Fired when a key is released.
    ///
    /// # SDK Example
    /// ```rust
    /// if event.event_kind == 17 {
    ///     // Stop key-held plugin behavior.
    /// }
    /// ```
    KeyReleased,
    /// Fired when the active overlay changes.
    ///
    /// # SDK Example
    /// ```rust
    /// if event.event_kind == 18 {
    ///     // Read event.overlay_id from the ABI payload.
    /// }
    /// ```
    OverlayChanged,
    /// Fired when the HUD is built for the hovered cell.
    ///
    /// # SDK Example
    /// ```rust
    /// if event.event_kind == 19 && event.has_cell != 0 {
    ///     // Add a HUD block with submit_hud_block.
    /// }
    /// ```
    BuildHudForCell,
    /// Fired when a plugin-owned panel should be built.
    ///
    /// # SDK Example
    /// ```rust
    /// if event.event_kind == 20 {
    ///     // Emit or refresh plugin panel UI state.
    /// }
    /// ```
    BuildPanel,
    /// Fired when a plugin-controlled overlay should submit a frame.
    ///
    /// # SDK Example
    /// ```rust
    /// if event.event_kind == 21 {
    ///     // Submit a 102x102 RGBA8 frame with submit_overlay_frame.
    /// }
    /// ```
    RenderOverlay,
}

/// Keyboard modifier state carried by plugin input events.
///
/// # Fields
/// Public fields of `InputModifiers` are part of the generated SDK reference.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

/// Mouse button used by plugin cell input events.
///
/// # Variants
/// Public variants of `MouseCellButton` are listed in the Rust declaration and documented by the generated SDK reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseCellButton {
    Left,
    Right,
    Middle,
    Other(u16),
}

/// Low-level mouse event in a world cell.
///
/// # Fields
/// Public fields of `MouseCellEvent` are part of the generated SDK reference.
#[derive(Clone, Debug, PartialEq)]
pub struct MouseCellEvent {
    pub button: Option<MouseCellButton>,
    pub cell: UVec2,
    pub world_position: Vec2,
    pub screen_position: Vec2,
    pub modifiers: InputModifiers,
    pub active_tool_id: Option<ContentId>,
    pub is_over_ui: bool,
}

/// Structure lifecycle event payload.
///
/// # Fields
/// Public fields of `StructureEvent` are part of the generated SDK reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructureEvent {
    pub id: PlacedStructureId,
    pub kind: StructureKind,
    pub cell: UVec2,
}

/// Runtime event payload sent through the Rust plugin API.
///
/// # Variants
/// Public variants of `PluginEvent` are listed in the Rust declaration and documented by the generated SDK reference.
#[derive(Event, Clone, Debug, PartialEq)]
pub enum PluginEvent {
    WorldCreated,
    WorldLoaded,
    WorldBeforeSave,
    WorldAfterSave,
    WorldUnloaded,
    SimulationPreCellGasStep,
    SimulationPostCellGasStep,
    SimulationPausedChanged {
        paused: bool,
    },
    StructurePlaced(StructureEvent),
    StructureRemoved(StructureEvent),
    ToolSelected {
        tool_id: Option<ContentId>,
    },
    MouseDownCell(MouseCellEvent),
    MouseMoveCell(MouseCellEvent),
    MouseUpCell(MouseCellEvent),
    MouseEnterCell(MouseCellEvent),
    MouseLeaveCell(MouseCellEvent),
    KeyPressed {
        key: String,
        modifiers: InputModifiers,
    },
    KeyReleased {
        key: String,
        modifiers: InputModifiers,
    },
    OverlayChanged {
        overlay_id: Option<ContentId>,
    },
    BuildHudForCell {
        cell: UVec2,
    },
    BuildPanel {
        panel_id: ContentId,
    },
    RenderOverlay {
        overlay_id: ContentId,
    },
}

impl PluginEvent {
    /// Returns the subscription category for this event.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `kind` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn kind(&self) -> PluginEventKind {
        match self {
            Self::WorldCreated => PluginEventKind::WorldCreated,
            Self::WorldLoaded => PluginEventKind::WorldLoaded,
            Self::WorldBeforeSave => PluginEventKind::WorldBeforeSave,
            Self::WorldAfterSave => PluginEventKind::WorldAfterSave,
            Self::WorldUnloaded => PluginEventKind::WorldUnloaded,
            Self::SimulationPreCellGasStep => PluginEventKind::SimulationPreCellGasStep,
            Self::SimulationPostCellGasStep => PluginEventKind::SimulationPostCellGasStep,
            Self::SimulationPausedChanged { .. } => PluginEventKind::SimulationPausedChanged,
            Self::StructurePlaced(_) => PluginEventKind::StructurePlaced,
            Self::StructureRemoved(_) => PluginEventKind::StructureRemoved,
            Self::ToolSelected { .. } => PluginEventKind::ToolSelected,
            Self::MouseDownCell(_) => PluginEventKind::MouseDownCell,
            Self::MouseMoveCell(_) => PluginEventKind::MouseMoveCell,
            Self::MouseUpCell(_) => PluginEventKind::MouseUpCell,
            Self::MouseEnterCell(_) => PluginEventKind::MouseEnterCell,
            Self::MouseLeaveCell(_) => PluginEventKind::MouseLeaveCell,
            Self::KeyPressed { .. } => PluginEventKind::KeyPressed,
            Self::KeyReleased { .. } => PluginEventKind::KeyReleased,
            Self::OverlayChanged { .. } => PluginEventKind::OverlayChanged,
            Self::BuildHudForCell { .. } => PluginEventKind::BuildHudForCell,
            Self::BuildPanel { .. } => PluginEventKind::BuildPanel,
            Self::RenderOverlay { .. } => PluginEventKind::RenderOverlay,
        }
    }
}
