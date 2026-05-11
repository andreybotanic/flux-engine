use std::ffi::c_void;

use crate::plugins::{api::events::PluginEventKind, id::ENGINE_PLUGIN_API_VERSION_VALUE};

use super::{FluxPluginHandle, FluxRuntimeHost, FluxStatus, FluxUtf8Slice};

/// Stable plugin-visible event kind used by ABI v4 registration helpers.
///
/// The raw numeric tag is part of the DLL ABI, but plugins should prefer this
/// enum over hardcoded integers when declaring handlers.
///
/// # Variants
/// - `WorldCreated`: Raw ABI tag for the fresh-world lifecycle event.
/// - `WorldLoaded`: Raw ABI tag for the world-loaded lifecycle event.
/// - `WorldBeforeSave`: Raw ABI tag for the pre-save lifecycle event.
/// - `WorldAfterSave`: Raw ABI tag for the post-save lifecycle event.
/// - `WorldUnloaded`: Raw ABI tag for the world-unloaded lifecycle event.
/// - `SimulationPreCellGasStep`: Raw ABI tag for the pre-simulation gas tick event.
/// - `SimulationPostCellGasStep`: Raw ABI tag for the post-simulation gas tick event.
/// - `SimulationPausedChanged`: Raw ABI tag for the pause-state change event.
/// - `StructurePlaced`: Raw ABI tag for the structure-placed event.
/// - `StructureRemoved`: Raw ABI tag for the structure-removed event.
/// - `ToolSelected`: Raw ABI tag for the tool-selected event.
/// - `MouseDownCell`: Raw ABI tag for the mouse-button-down cell event.
/// - `MouseMoveCell`: Raw ABI tag for the mouse-move cell event.
/// - `MouseUpCell`: Raw ABI tag for the mouse-button-up cell event.
/// - `MouseEnterCell`: Raw ABI tag for the mouse-enter cell event.
/// - `MouseLeaveCell`: Raw ABI tag for the mouse-leave cell event.
/// - `KeyPressed`: Raw ABI tag for the key-pressed event.
/// - `KeyReleased`: Raw ABI tag for the key-released event.
/// - `OverlayChanged`: Raw ABI tag for the overlay-changed event.
/// - `BuildHudForCell`: Raw ABI tag for the HUD-build event.
/// - `BuildPanel`: Raw ABI tag for the panel-build event.
/// - `RenderOverlay`: Raw ABI tag for the overlay-render event.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FluxEventKind {
    WorldCreated = 0,
    WorldLoaded = 1,
    WorldBeforeSave = 2,
    WorldAfterSave = 3,
    WorldUnloaded = 4,
    SimulationPreCellGasStep = 5,
    SimulationPostCellGasStep = 6,
    SimulationPausedChanged = 7,
    StructurePlaced = 8,
    StructureRemoved = 9,
    ToolSelected = 10,
    MouseDownCell = 11,
    MouseMoveCell = 12,
    MouseUpCell = 13,
    MouseEnterCell = 14,
    MouseLeaveCell = 15,
    KeyPressed = 16,
    KeyReleased = 17,
    OverlayChanged = 18,
    BuildHudForCell = 19,
    BuildPanel = 20,
    RenderOverlay = 21,
}

impl FluxEventKind {
    /// Returns the stable raw ABI tag used on the DLL boundary.
    ///
    pub fn as_raw(self) -> u32 {
        self as u32
    }

    /// Parses one raw ABI tag into a typed plugin-visible event kind.
    ///
    pub fn from_raw(value: u32) -> Option<Self> {
        Some(match value {
            0 => Self::WorldCreated,
            1 => Self::WorldLoaded,
            2 => Self::WorldBeforeSave,
            3 => Self::WorldAfterSave,
            4 => Self::WorldUnloaded,
            5 => Self::SimulationPreCellGasStep,
            6 => Self::SimulationPostCellGasStep,
            7 => Self::SimulationPausedChanged,
            8 => Self::StructurePlaced,
            9 => Self::StructureRemoved,
            10 => Self::ToolSelected,
            11 => Self::MouseDownCell,
            12 => Self::MouseMoveCell,
            13 => Self::MouseUpCell,
            14 => Self::MouseEnterCell,
            15 => Self::MouseLeaveCell,
            16 => Self::KeyPressed,
            17 => Self::KeyReleased,
            18 => Self::OverlayChanged,
            19 => Self::BuildHudForCell,
            20 => Self::BuildPanel,
            21 => Self::RenderOverlay,
            _ => return None,
        })
    }

    fn from_engine(kind: PluginEventKind) -> Self {
        match kind {
            PluginEventKind::WorldCreated => Self::WorldCreated,
            PluginEventKind::WorldLoaded => Self::WorldLoaded,
            PluginEventKind::WorldBeforeSave => Self::WorldBeforeSave,
            PluginEventKind::WorldAfterSave => Self::WorldAfterSave,
            PluginEventKind::WorldUnloaded => Self::WorldUnloaded,
            PluginEventKind::SimulationPreCellGasStep => Self::SimulationPreCellGasStep,
            PluginEventKind::SimulationPostCellGasStep => Self::SimulationPostCellGasStep,
            PluginEventKind::SimulationPausedChanged => Self::SimulationPausedChanged,
            PluginEventKind::StructurePlaced => Self::StructurePlaced,
            PluginEventKind::StructureRemoved => Self::StructureRemoved,
            PluginEventKind::ToolSelected => Self::ToolSelected,
            PluginEventKind::MouseDownCell => Self::MouseDownCell,
            PluginEventKind::MouseMoveCell => Self::MouseMoveCell,
            PluginEventKind::MouseUpCell => Self::MouseUpCell,
            PluginEventKind::MouseEnterCell => Self::MouseEnterCell,
            PluginEventKind::MouseLeaveCell => Self::MouseLeaveCell,
            PluginEventKind::KeyPressed => Self::KeyPressed,
            PluginEventKind::KeyReleased => Self::KeyReleased,
            PluginEventKind::OverlayChanged => Self::OverlayChanged,
            PluginEventKind::BuildHudForCell => Self::BuildHudForCell,
            PluginEventKind::BuildPanel => Self::BuildPanel,
            PluginEventKind::RenderOverlay => Self::RenderOverlay,
        }
    }

    fn into_engine(self) -> PluginEventKind {
        match self {
            Self::WorldCreated => PluginEventKind::WorldCreated,
            Self::WorldLoaded => PluginEventKind::WorldLoaded,
            Self::WorldBeforeSave => PluginEventKind::WorldBeforeSave,
            Self::WorldAfterSave => PluginEventKind::WorldAfterSave,
            Self::WorldUnloaded => PluginEventKind::WorldUnloaded,
            Self::SimulationPreCellGasStep => PluginEventKind::SimulationPreCellGasStep,
            Self::SimulationPostCellGasStep => PluginEventKind::SimulationPostCellGasStep,
            Self::SimulationPausedChanged => PluginEventKind::SimulationPausedChanged,
            Self::StructurePlaced => PluginEventKind::StructurePlaced,
            Self::StructureRemoved => PluginEventKind::StructureRemoved,
            Self::ToolSelected => PluginEventKind::ToolSelected,
            Self::MouseDownCell => PluginEventKind::MouseDownCell,
            Self::MouseMoveCell => PluginEventKind::MouseMoveCell,
            Self::MouseUpCell => PluginEventKind::MouseUpCell,
            Self::MouseEnterCell => PluginEventKind::MouseEnterCell,
            Self::MouseLeaveCell => PluginEventKind::MouseLeaveCell,
            Self::KeyPressed => PluginEventKind::KeyPressed,
            Self::KeyReleased => PluginEventKind::KeyReleased,
            Self::OverlayChanged => PluginEventKind::OverlayChanged,
            Self::BuildHudForCell => PluginEventKind::BuildHudForCell,
            Self::BuildPanel => PluginEventKind::BuildPanel,
            Self::RenderOverlay => PluginEventKind::RenderOverlay,
        }
    }

    fn canonical_handler_name(self) -> &'static str {
        match self {
            Self::WorldCreated => "onWorldCreated",
            Self::WorldLoaded => "onWorldLoaded",
            Self::WorldBeforeSave => "onWorldBeforeSave",
            Self::WorldAfterSave => "onWorldAfterSave",
            Self::WorldUnloaded => "onWorldUnloaded",
            Self::SimulationPreCellGasStep => "onSimulationPreCellGasStep",
            Self::SimulationPostCellGasStep => "onSimulationPostCellGasStep",
            Self::SimulationPausedChanged => "onSimulationPausedChanged",
            Self::StructurePlaced => "onStructurePlaced",
            Self::StructureRemoved => "onStructureRemoved",
            Self::ToolSelected => "onToolSelected",
            Self::MouseDownCell => "onMouseDownCell",
            Self::MouseMoveCell => "onMouseMoveCell",
            Self::MouseUpCell => "onMouseUpCell",
            Self::MouseEnterCell => "onMouseEnterCell",
            Self::MouseLeaveCell => "onMouseLeaveCell",
            Self::KeyPressed => "onKeyPressed",
            Self::KeyReleased => "onKeyReleased",
            Self::OverlayChanged => "onOverlayChanged",
            Self::BuildHudForCell => "onBuildHudForCell",
            Self::BuildPanel => "onBuildPanel",
            Self::RenderOverlay => "onRenderOverlay",
        }
    }
}

/// Stable ABI descriptor that binds one plugin event kind to one named handler export.
///
/// # Fields
/// - `event_kind`: Raw ABI event tag created from `FluxEventKind::as_raw()`.
/// - `handler_name`: UTF-8 export name of the plugin callback that handles this event.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxEventHandlerDescriptor {
    /// Raw ABI tag produced from `FluxEventKind::as_raw()`.
    pub event_kind: u32,
    /// Name of the plugin export that handles this event kind.
    pub handler_name: FluxUtf8Slice,
}

impl FluxEventHandlerDescriptor {
    /// Creates one event-handler descriptor from the typed plugin-visible event enum.
    ///
    pub fn new(event_kind: FluxEventKind, handler_name: FluxUtf8Slice) -> Self {
        Self {
            event_kind: event_kind.as_raw(),
            handler_name,
        }
    }
}

/// Callback used by plugins to register one explicit event-handler binding.
///
pub type FluxRegisterEventHandlerFn = unsafe extern "C" fn(
    context: *mut c_void,
    descriptor: *const FluxEventHandlerDescriptor,
) -> FluxStatus;

/// Empty payload shared by lifecycle and tick events that do not carry extra fields.
///
/// # Fields
/// - `struct_size`: Size of this payload struct used for ABI validation.
/// - `api_version`: ABI version expected by the event producer and consumer.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxEmptyEventPayload {
    pub struct_size: u32,
    pub api_version: u32,
}

impl FluxEmptyEventPayload {
    /// Creates one empty event payload for ABI v4 event callbacks.
    ///
    pub fn new() -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>() as u32,
            api_version: ENGINE_PLUGIN_API_VERSION_VALUE,
        }
    }
}

/// Payload emitted when the simulation pause flag changes.
///
/// # Fields
/// - `struct_size`: Size of this payload struct used for ABI validation.
/// - `api_version`: ABI version expected by the event producer and consumer.
/// - `paused`: New pause flag encoded as `0` or `1`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxSimulationPausedChangedEvent {
    pub struct_size: u32,
    pub api_version: u32,
    pub paused: u8,
}

/// Payload emitted for structure placement and removal events.
///
/// # Fields
/// - `struct_size`: Size of this payload struct used for ABI validation.
/// - `api_version`: ABI version expected by the event producer and consumer.
/// - `structure_id`: Runtime id of the affected structure instance.
/// - `structure_kind`: Stable structure kind id of the affected instance.
/// - `cell_x`: X coordinate of the primary structure cell.
/// - `cell_y`: Y coordinate of the primary structure cell.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxStructureEventPayload {
    pub struct_size: u32,
    pub api_version: u32,
    pub structure_id: u32,
    pub structure_kind: FluxUtf8Slice,
    pub cell_x: u32,
    pub cell_y: u32,
}

/// Payload emitted for low-level mouse interaction over world cells.
///
/// # Fields
/// - `struct_size`: Size of this payload struct used for ABI validation.
/// - `api_version`: ABI version expected by the event producer and consumer.
/// - `button`: Encoded mouse button id associated with the event.
/// - `has_cell`: Whether `cell_x` and `cell_y` contain a valid world-cell target.
/// - `cell_x`: X coordinate of the targeted world cell.
/// - `cell_y`: Y coordinate of the targeted world cell.
/// - `world_x`: World-space cursor X coordinate.
/// - `world_y`: World-space cursor Y coordinate.
/// - `screen_x`: Screen-space cursor X coordinate.
/// - `screen_y`: Screen-space cursor Y coordinate.
/// - `modifiers`: Bitset of keyboard modifiers held during the event.
/// - `has_active_tool_id`: Whether `active_tool_id` contains a selected tool id.
/// - `active_tool_id`: Stable content id of the active tool, when present.
/// - `is_over_ui`: Whether the pointer was over UI when the event fired.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxMouseCellEventPayload {
    pub struct_size: u32,
    pub api_version: u32,
    pub button: u32,
    pub has_cell: u8,
    pub cell_x: u32,
    pub cell_y: u32,
    pub world_x: f32,
    pub world_y: f32,
    pub screen_x: f32,
    pub screen_y: f32,
    pub modifiers: u32,
    pub has_active_tool_id: u8,
    pub active_tool_id: FluxUtf8Slice,
    pub is_over_ui: u8,
}

/// Payload emitted for key press and key release events.
///
/// # Fields
/// - `struct_size`: Size of this payload struct used for ABI validation.
/// - `api_version`: ABI version expected by the event producer and consumer.
/// - `key`: UTF-8 key identifier emitted by the engine.
/// - `modifiers`: Bitset of keyboard modifiers held during the event.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxKeyEventPayload {
    pub struct_size: u32,
    pub api_version: u32,
    pub key: FluxUtf8Slice,
    pub modifiers: u32,
}

/// Payload emitted when the active editor tool changes.
///
/// # Fields
/// - `struct_size`: Size of this payload struct used for ABI validation.
/// - `api_version`: ABI version expected by the event producer and consumer.
/// - `has_tool_id`: Whether `tool_id` contains a valid active tool identifier.
/// - `tool_id`: Stable content id of the newly active tool, when present.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxToolSelectedEventPayload {
    pub struct_size: u32,
    pub api_version: u32,
    pub has_tool_id: u8,
    pub tool_id: FluxUtf8Slice,
}

/// Payload emitted when the active overlay changes.
///
/// # Fields
/// - `struct_size`: Size of this payload struct used for ABI validation.
/// - `api_version`: ABI version expected by the event producer and consumer.
/// - `has_overlay_id`: Whether `overlay_id` contains a plugin-owned overlay identifier.
/// - `overlay_id`: Stable content id of the newly active overlay, when present.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxOverlayChangedEventPayload {
    pub struct_size: u32,
    pub api_version: u32,
    pub has_overlay_id: u8,
    pub overlay_id: FluxUtf8Slice,
}

/// Payload emitted when plugins can append HUD blocks for one cell.
///
/// # Fields
/// - `struct_size`: Size of this payload struct used for ABI validation.
/// - `api_version`: ABI version expected by the event producer and consumer.
/// - `cell_x`: X coordinate of the hovered world cell.
/// - `cell_y`: Y coordinate of the hovered world cell.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxBuildHudForCellEventPayload {
    pub struct_size: u32,
    pub api_version: u32,
    pub cell_x: u32,
    pub cell_y: u32,
}

/// Payload emitted when a plugin-owned panel should be built or refreshed.
///
/// # Fields
/// - `struct_size`: Size of this payload struct used for ABI validation.
/// - `api_version`: ABI version expected by the event producer and consumer.
/// - `panel_id`: Stable content id of the panel being requested.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxBuildPanelEventPayload {
    pub struct_size: u32,
    pub api_version: u32,
    pub panel_id: FluxUtf8Slice,
}

/// Payload emitted when a plugin-controlled overlay should render a frame.
///
/// # Fields
/// - `struct_size`: Size of this payload struct used for ABI validation.
/// - `api_version`: ABI version expected by the event producer and consumer.
/// - `overlay_id`: Stable content id of the overlay that should render a frame.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxRenderOverlayEventPayload {
    pub struct_size: u32,
    pub api_version: u32,
    pub overlay_id: FluxUtf8Slice,
}

/// Function pointer type for `onWorldCreated`.
///
pub type FluxOnWorldCreatedFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxEmptyEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onWorldLoaded`.
///
pub type FluxOnWorldLoadedFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxEmptyEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onWorldBeforeSave`.
///
pub type FluxOnWorldBeforeSaveFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxEmptyEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onWorldAfterSave`.
///
pub type FluxOnWorldAfterSaveFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxEmptyEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onWorldUnloaded`.
///
pub type FluxOnWorldUnloadedFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxEmptyEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onSimulationPreCellGasStep`.
///
pub type FluxOnSimulationPreCellGasStepFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxEmptyEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onSimulationPostCellGasStep`.
///
pub type FluxOnSimulationPostCellGasStepFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxEmptyEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onSimulationPausedChanged`.
///
pub type FluxOnSimulationPausedChangedFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxSimulationPausedChangedEvent,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onStructurePlaced`.
///
pub type FluxOnStructurePlacedFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxStructureEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onStructureRemoved`.
///
pub type FluxOnStructureRemovedFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxStructureEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onToolSelected`.
///
pub type FluxOnToolSelectedFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxToolSelectedEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onMouseDownCell`.
///
pub type FluxOnMouseDownCellFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxMouseCellEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onMouseMoveCell`.
///
pub type FluxOnMouseMoveCellFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxMouseCellEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onMouseUpCell`.
///
pub type FluxOnMouseUpCellFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxMouseCellEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onMouseEnterCell`.
///
pub type FluxOnMouseEnterCellFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxMouseCellEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onMouseLeaveCell`.
///
pub type FluxOnMouseLeaveCellFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxMouseCellEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onKeyPressed`.
///
pub type FluxOnKeyPressedFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxKeyEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onKeyReleased`.
///
pub type FluxOnKeyReleasedFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxKeyEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onOverlayChanged`.
///
pub type FluxOnOverlayChangedFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxOverlayChangedEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onBuildHudForCell`.
///
pub type FluxOnBuildHudForCellFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxBuildHudForCellEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onBuildPanel`.
///
pub type FluxOnBuildPanelFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxBuildPanelEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onRenderOverlay`.
///
pub type FluxOnRenderOverlayFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxRenderOverlayEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Returns the stable ABI numeric tag for one engine-side plugin event kind.
///
pub fn event_kind_to_abi(kind: PluginEventKind) -> u32 {
    FluxEventKind::from_engine(kind).as_raw()
}

/// Converts one stable ABI numeric tag back into the engine-side event kind.
///
pub fn event_kind_from_abi(value: u32) -> Option<PluginEventKind> {
    Some(FluxEventKind::from_raw(value)?.into_engine())
}

/// Returns the canonical handler name used in docs and demo plugins for one event kind.
///
pub fn canonical_handler_name(kind: PluginEventKind) -> &'static str {
    FluxEventKind::from_engine(kind).canonical_handler_name()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flux_event_kind_roundtrips_known_values() {
        let kind = FluxEventKind::KeyPressed;

        assert_eq!(kind.as_raw(), 16);
        assert_eq!(FluxEventKind::from_raw(16), Some(kind));
        assert_eq!(event_kind_to_abi(PluginEventKind::KeyPressed), 16);
        assert_eq!(event_kind_from_abi(16), Some(PluginEventKind::KeyPressed));
        assert_eq!(
            canonical_handler_name(PluginEventKind::KeyPressed),
            "onKeyPressed"
        );
    }

    #[test]
    fn flux_event_kind_rejects_unknown_raw_values() {
        assert_eq!(FluxEventKind::from_raw(u32::MAX), None);
        assert_eq!(event_kind_from_abi(u32::MAX), None);
    }

    #[test]
    fn flux_event_handler_descriptor_new_uses_typed_event_kind() {
        let descriptor = FluxEventHandlerDescriptor::new(
            FluxEventKind::MouseDownCell,
            FluxUtf8Slice::from_str("onMouseDownCell"),
        );

        assert_eq!(descriptor.event_kind, 11);
        assert_eq!(descriptor.handler_name.len, "onMouseDownCell".len());
    }
}
