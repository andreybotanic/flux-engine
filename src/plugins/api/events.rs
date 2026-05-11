use bevy::prelude::*;

use crate::{
    plugins::ContentId,
    world::structures::{PlacedStructureId, StructureKind},
};

/// Plugin-visible event category.
///
/// # Variants
/// - `WorldCreated`: Fired immediately after the engine creates a fresh world state.
/// - `WorldLoaded`: Fired after a saved world and its plugin state are loaded.
/// - `WorldBeforeSave`: Fired synchronously before the engine serializes the current world.
/// - `WorldAfterSave`: Fired after a save pass completes.
/// - `WorldUnloaded`: Fired before the current world is dropped from memory.
/// - `SimulationPreCellGasStep`: Fired before the core free-gas simulation step begins.
/// - `SimulationPostCellGasStep`: Fired after the core free-gas simulation step completes.
/// - `SimulationPausedChanged`: Fired whenever the simulation pause flag toggles.
/// - `StructurePlaced`: Fired after a structure instance is placed into the world.
/// - `StructureRemoved`: Fired after a structure instance is removed from the world.
/// - `ToolSelected`: Fired when the active editor tool changes.
/// - `MouseDownCell`: Fired when a mouse button is pressed over a world cell.
/// - `MouseMoveCell`: Fired when the cursor moves across world cells.
/// - `MouseUpCell`: Fired when a mouse button is released over a world cell.
/// - `MouseEnterCell`: Fired when the cursor enters a world cell.
/// - `MouseLeaveCell`: Fired when the cursor leaves a world cell.
/// - `KeyPressed`: Fired when a keyboard key is pressed.
/// - `KeyReleased`: Fired when a keyboard key is released.
/// - `OverlayChanged`: Fired when the active overlay mode changes.
/// - `BuildHudForCell`: Fired when plugins can append HUD blocks for the hovered cell.
/// - `BuildPanel`: Fired when a plugin-owned panel should be built or refreshed.
/// - `RenderOverlay`: Fired when a plugin-controlled overlay should produce a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PluginEvent {
    /// Fired after a new world is created.
    ///
    WorldCreated,
    /// Fired after a save slot has been loaded.
    ///
    WorldLoaded,
    /// Fired synchronously before the current world is saved.
    ///
    WorldBeforeSave,
    /// Fired after a save operation finishes.
    ///
    WorldAfterSave,
    /// Fired before the current world is unloaded.
    ///
    WorldUnloaded,
    /// Fired before the core free-gas simulation step.
    ///
    SimulationPreCellGasStep,
    /// Fired after the core free-gas simulation step.
    ///
    SimulationPostCellGasStep,
    /// Fired when the simulation pause state changes.
    ///
    SimulationPausedChanged,
    /// Fired after a structure is placed.
    ///
    StructurePlaced,
    /// Fired after a structure is removed.
    ///
    StructureRemoved,
    /// Fired when the active editor tool changes.
    ///
    ToolSelected,
    /// Fired when a mouse button is pressed over a world cell.
    ///
    MouseDownCell,
    /// Fired when the cursor moves over world cells.
    ///
    MouseMoveCell,
    /// Fired when a mouse button is released over a world cell.
    ///
    MouseUpCell,
    /// Fired when the cursor enters a world cell.
    ///
    MouseEnterCell,
    /// Fired when the cursor leaves a world cell.
    ///
    MouseLeaveCell,
    /// Fired when a key is pressed.
    ///
    KeyPressed,
    /// Fired when a key is released.
    ///
    KeyReleased,
    /// Fired when the active overlay changes.
    ///
    OverlayChanged,
    /// Fired when the HUD is built for the hovered cell.
    ///
    BuildHudForCell,
    /// Fired when a plugin-owned panel should be built.
    ///
    BuildPanel,
    /// Fired when a plugin-controlled overlay should submit a frame.
    ///
    RenderOverlay,
}

/// Keyboard modifier state carried by plugin input events.
///
/// # Fields
/// - `shift`: Whether either Shift key was held when the input event fired.
/// - `ctrl`: Whether either Control key was held when the input event fired.
/// - `alt`: Whether either Alt key was held when the input event fired.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

/// Mouse button used by plugin cell input events.
///
/// # Variants
/// - `Left`: Primary mouse button.
/// - `Right`: Secondary mouse button.
/// - `Middle`: Middle or wheel mouse button.
/// - `Other`: Additional mouse button encoded by its platform-provided numeric id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u16),
}

/// Low-level mouse event in a world cell.
///
/// # Fields
/// - `button`: Mouse button associated with the event, if the source event had one.
/// - `cell`: Target world-cell coordinates under the cursor.
/// - `world_position`: Cursor position in world-space coordinates.
/// - `screen_position`: Cursor position in screen-space coordinates.
/// - `modifiers`: Keyboard modifier snapshot captured with the mouse event.
/// - `active_tool_id`: Active editor tool when the event fired, if one is selected.
/// - `is_over_ui`: Whether the pointer was over game UI when the event was emitted.
#[derive(Clone, Debug, PartialEq)]
pub struct MouseCellEvent {
    pub button: Option<MouseButton>,
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
/// - `id`: Runtime identifier of the structure instance that changed.
/// - `kind`: Registered structure kind of the affected instance.
/// - `cell`: Primary world-cell location associated with the structure lifecycle event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructureEvent {
    pub id: PlacedStructureId,
    pub kind: StructureKind,
    pub cell: UVec2,
}

/// Runtime event payload sent through the engine-side runtime event queue.
///
/// # Variants
/// - `WorldCreated`: World-lifecycle event with no additional payload.
/// - `WorldLoaded`: World-lifecycle event with no additional payload.
/// - `WorldBeforeSave`: Save-lifecycle event with no additional payload.
/// - `WorldAfterSave`: Save-lifecycle event with no additional payload.
/// - `WorldUnloaded`: World-lifecycle event with no additional payload.
/// - `SimulationPreCellGasStep`: Simulation tick event emitted before free-gas processing.
/// - `SimulationPostCellGasStep`: Simulation tick event emitted after free-gas processing.
/// - `SimulationPausedChanged`: Carries the new pause flag after a pause/resume transition.
/// - `StructurePlaced`: Carries the placed structure payload.
/// - `StructureRemoved`: Carries the removed structure payload.
/// - `ToolSelected`: Carries the newly selected tool id, if any.
/// - `MouseDownCell`: Carries the low-level mouse payload for a button press.
/// - `MouseMoveCell`: Carries the low-level mouse payload for cursor movement.
/// - `MouseUpCell`: Carries the low-level mouse payload for a button release.
/// - `MouseEnterCell`: Carries the low-level mouse payload for cell entry.
/// - `MouseLeaveCell`: Carries the low-level mouse payload for cell exit.
/// - `KeyPressed`: Carries the pressed key string and modifier snapshot.
/// - `KeyReleased`: Carries the released key string and modifier snapshot.
/// - `OverlayChanged`: Carries the new overlay id, if the active overlay is plugin-owned.
/// - `BuildHudForCell`: Carries the hovered world cell for HUD augmentation.
/// - `BuildPanel`: Carries the plugin-owned panel id being requested.
/// - `RenderOverlay`: Carries the plugin-owned overlay id that should render a frame.
#[derive(Event, Clone, Debug, PartialEq)]
pub(crate) enum PluginRuntimeEvent {
    WorldCreated,
    WorldLoaded,
    WorldBeforeSave,
    WorldAfterSave,
    WorldUnloaded,
    SimulationPreCellGasStep,
    SimulationPostCellGasStep,
    SimulationPausedChanged {
        /// New pause flag after the transition completes.
        paused: bool,
    },
    StructurePlaced(
        /// Structure payload describing the placed instance.
        StructureEvent,
    ),
    StructureRemoved(
        /// Structure payload describing the removed instance.
        StructureEvent,
    ),
    ToolSelected {
        /// Newly selected tool id, or `None` when no tool is active.
        tool_id: Option<ContentId>,
    },
    MouseDownCell(
        /// Low-level mouse payload captured for the button press.
        MouseCellEvent,
    ),
    MouseMoveCell(
        /// Low-level mouse payload captured for cursor movement.
        MouseCellEvent,
    ),
    MouseUpCell(
        /// Low-level mouse payload captured for the button release.
        MouseCellEvent,
    ),
    MouseEnterCell(
        /// Low-level mouse payload captured when the cursor entered a cell.
        MouseCellEvent,
    ),
    MouseLeaveCell(
        /// Low-level mouse payload captured when the cursor left a cell.
        MouseCellEvent,
    ),
    KeyPressed {
        /// Engine-provided key identifier for the pressed key.
        key: String,
        /// Modifier snapshot captured together with the key press.
        modifiers: InputModifiers,
    },
    KeyReleased {
        /// Engine-provided key identifier for the released key.
        key: String,
        /// Modifier snapshot captured together with the key release.
        modifiers: InputModifiers,
    },
    OverlayChanged {
        /// Newly active plugin-owned overlay id, if one is selected.
        overlay_id: Option<ContentId>,
    },
    BuildHudForCell {
        /// Hovered world cell that the plugin can augment in the HUD.
        cell: UVec2,
    },
    BuildPanel {
        /// Plugin-owned panel id being requested by the UI.
        panel_id: ContentId,
    },
    RenderOverlay {
        /// Plugin-owned overlay id that should submit a frame.
        overlay_id: ContentId,
    },
}

impl PluginRuntimeEvent {
    /// Returns the subscription category for this runtime event instance.
    ///
    pub(crate) fn kind(&self) -> PluginEvent {
        match self {
            Self::WorldCreated => PluginEvent::WorldCreated,
            Self::WorldLoaded => PluginEvent::WorldLoaded,
            Self::WorldBeforeSave => PluginEvent::WorldBeforeSave,
            Self::WorldAfterSave => PluginEvent::WorldAfterSave,
            Self::WorldUnloaded => PluginEvent::WorldUnloaded,
            Self::SimulationPreCellGasStep => PluginEvent::SimulationPreCellGasStep,
            Self::SimulationPostCellGasStep => PluginEvent::SimulationPostCellGasStep,
            Self::SimulationPausedChanged { .. } => PluginEvent::SimulationPausedChanged,
            Self::StructurePlaced(_) => PluginEvent::StructurePlaced,
            Self::StructureRemoved(_) => PluginEvent::StructureRemoved,
            Self::ToolSelected { .. } => PluginEvent::ToolSelected,
            Self::MouseDownCell(_) => PluginEvent::MouseDownCell,
            Self::MouseMoveCell(_) => PluginEvent::MouseMoveCell,
            Self::MouseUpCell(_) => PluginEvent::MouseUpCell,
            Self::MouseEnterCell(_) => PluginEvent::MouseEnterCell,
            Self::MouseLeaveCell(_) => PluginEvent::MouseLeaveCell,
            Self::KeyPressed { .. } => PluginEvent::KeyPressed,
            Self::KeyReleased { .. } => PluginEvent::KeyReleased,
            Self::OverlayChanged { .. } => PluginEvent::OverlayChanged,
            Self::BuildHudForCell { .. } => PluginEvent::BuildHudForCell,
            Self::BuildPanel { .. } => PluginEvent::BuildPanel,
            Self::RenderOverlay { .. } => PluginEvent::RenderOverlay,
        }
    }
}
