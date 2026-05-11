use std::ffi::c_void;

use crate::plugins::{api::events::PluginEventKind, id::ENGINE_PLUGIN_API_VERSION_VALUE};

use super::{FluxPluginHandle, FluxRuntimeHost, FluxStatus, FluxUtf8Slice};

/// Stable plugin-visible event kind used by ABI v4 registration helpers.
///
/// The raw numeric tag is part of the DLL ABI, but plugins should prefer this
/// enum over hardcoded integers when declaring handlers.
///
/// # Variants
/// Public variants of `FluxEventKind` are listed in the Rust declaration and documented by the generated SDK reference.
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
    /// # SDK Example
    /// ```rust
    /// // Convert a typed plugin-visible event kind into the raw ABI tag when needed.
    /// ```
    pub fn as_raw(self) -> u32 {
        self as u32
    }

    /// Parses one raw ABI tag into a typed plugin-visible event kind.
    ///
    /// # SDK Example
    /// ```rust
    /// // Convert a raw ABI tag back into `FluxEventKind` when validating untyped input.
    /// ```
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
/// Public fields of `FluxEventHandlerDescriptor` are part of the generated SDK reference.
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
    /// # SDK Example
    /// ```rust
    /// // Build a descriptor without hardcoding the raw numeric event tag.
    /// ```
    pub fn new(event_kind: FluxEventKind, handler_name: FluxUtf8Slice) -> Self {
        Self {
            event_kind: event_kind.as_raw(),
            handler_name,
        }
    }
}

/// Callback used by plugins to register one explicit event-handler binding.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxRegisterEventHandlerFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxRegisterEventHandlerFn = unsafe extern "C" fn(
    context: *mut c_void,
    descriptor: *const FluxEventHandlerDescriptor,
) -> FluxStatus;

/// Empty payload shared by lifecycle and tick events that do not carry extra fields.
///
/// # Fields
/// Public fields of `FluxEmptyEventPayload` are part of the generated SDK reference.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxEmptyEventPayload {
    pub struct_size: u32,
    pub api_version: u32,
}

impl FluxEmptyEventPayload {
    /// Creates one empty event payload for ABI v4 event callbacks.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `new` from plugin-facing code when this operation is available in context.
    /// ```
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
/// Public fields of `FluxSimulationPausedChangedEvent` are part of the generated SDK reference.
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
/// Public fields of `FluxStructureEventPayload` are part of the generated SDK reference.
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
/// Public fields of `FluxMouseCellEventPayload` are part of the generated SDK reference.
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
/// Public fields of `FluxKeyEventPayload` are part of the generated SDK reference.
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
/// Public fields of `FluxToolSelectedEventPayload` are part of the generated SDK reference.
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
/// Public fields of `FluxOverlayChangedEventPayload` are part of the generated SDK reference.
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
/// Public fields of `FluxBuildHudForCellEventPayload` are part of the generated SDK reference.
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
/// Public fields of `FluxBuildPanelEventPayload` are part of the generated SDK reference.
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
/// Public fields of `FluxRenderOverlayEventPayload` are part of the generated SDK reference.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxRenderOverlayEventPayload {
    pub struct_size: u32,
    pub api_version: u32,
    pub overlay_id: FluxUtf8Slice,
}

/// Function pointer type for `onWorldCreated`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxOnWorldCreatedFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxOnWorldCreatedFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxEmptyEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onWorldLoaded`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxOnWorldLoadedFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxOnWorldLoadedFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxEmptyEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onWorldBeforeSave`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxOnWorldBeforeSaveFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxOnWorldBeforeSaveFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxEmptyEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onWorldAfterSave`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxOnWorldAfterSaveFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxOnWorldAfterSaveFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxEmptyEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onWorldUnloaded`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxOnWorldUnloadedFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxOnWorldUnloadedFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxEmptyEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onSimulationPreCellGasStep`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxOnSimulationPreCellGasStepFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxOnSimulationPreCellGasStepFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxEmptyEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onSimulationPostCellGasStep`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxOnSimulationPostCellGasStepFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxOnSimulationPostCellGasStepFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxEmptyEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onSimulationPausedChanged`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxOnSimulationPausedChangedFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxOnSimulationPausedChangedFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxSimulationPausedChangedEvent,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onStructurePlaced`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxOnStructurePlacedFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxOnStructurePlacedFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxStructureEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onStructureRemoved`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxOnStructureRemovedFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxOnStructureRemovedFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxStructureEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onToolSelected`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxOnToolSelectedFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxOnToolSelectedFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxToolSelectedEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onMouseDownCell`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxOnMouseDownCellFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxOnMouseDownCellFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxMouseCellEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onMouseMoveCell`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxOnMouseMoveCellFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxOnMouseMoveCellFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxMouseCellEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onMouseUpCell`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxOnMouseUpCellFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxOnMouseUpCellFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxMouseCellEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onMouseEnterCell`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxOnMouseEnterCellFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxOnMouseEnterCellFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxMouseCellEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onMouseLeaveCell`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxOnMouseLeaveCellFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxOnMouseLeaveCellFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxMouseCellEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onKeyPressed`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxOnKeyPressedFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxOnKeyPressedFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxKeyEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onKeyReleased`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxOnKeyReleasedFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxOnKeyReleasedFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxKeyEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onOverlayChanged`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxOnOverlayChangedFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxOnOverlayChangedFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxOverlayChangedEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onBuildHudForCell`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxOnBuildHudForCellFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxOnBuildHudForCellFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxBuildHudForCellEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onBuildPanel`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxOnBuildPanelFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxOnBuildPanelFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxBuildPanelEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Function pointer type for `onRenderOverlay`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxOnRenderOverlayFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxOnRenderOverlayFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxRenderOverlayEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Returns the stable ABI numeric tag for one engine-side plugin event kind.
///
/// # SDK Example
/// ```rust
/// // Call `event_kind_to_abi` from engine-side code when translating PluginEventKind values.
/// ```
pub fn event_kind_to_abi(kind: PluginEventKind) -> u32 {
    FluxEventKind::from_engine(kind).as_raw()
}

/// Converts one stable ABI numeric tag back into the engine-side event kind.
///
/// # SDK Example
/// ```rust
/// // Call `event_kind_from_abi` from engine-side code when validating event registrations.
/// ```
pub fn event_kind_from_abi(value: u32) -> Option<PluginEventKind> {
    Some(FluxEventKind::from_raw(value)?.into_engine())
}

/// Returns the canonical handler name used in docs and demo plugins for one event kind.
///
/// # SDK Example
/// ```rust
/// // Call `canonical_handler_name` from engine-side code when suggesting default handler exports.
/// ```
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
