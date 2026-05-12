use bevy_math::{UVec2, Vec2};
use flux_plugin_abi::{
    FluxBuildHudForCellEventPayload, FluxEmptyEventPayload, FluxEntityEventPayload,
    FluxEventKind, FluxKeyEventPayload, FluxMouseCellEventPayload,
    FluxOverlayChangedEventPayload, FluxRenderOverlayEventPayload,
    FluxSimulationPausedChangedEvent, FluxToolSelectedEventPayload, FluxUtf8Slice,
};

use crate::{ContentId, EntityInstanceId, EntityKindId, InputModifiers, MouseButton, OverlayModeId, PluginError};

/// Runtime event category used for subscriptions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PluginEvent {
    /// Fired after a new world has been created.
    WorldCreated,
    /// Fired after a world has finished loading.
    WorldLoaded,
    /// Fired right before the engine serializes world state.
    WorldBeforeSave,
    /// Fired after the engine has finished saving world state.
    WorldAfterSave,
    /// Fired before the current world is discarded.
    WorldUnloaded,
    /// Fired before one gas-simulation cell step starts.
    SimulationPreCellGasStep,
    /// Fired after one gas-simulation cell step completes.
    SimulationPostCellGasStep,
    /// Fired when pause state changes.
    SimulationPausedChanged,
    /// Fired after one entity has been placed into the world.
    EntityPlaced,
    /// Fired after one entity has been removed from the world.
    EntityRemoved,
    /// Fired when the active tool changes.
    ToolSelected,
    /// Fired when the pointer button is pressed over a world cell.
    MouseDownCell,
    /// Fired when the pointer moves over world cells.
    MouseMoveCell,
    /// Fired when the pointer button is released over a world cell.
    MouseUpCell,
    /// Fired when the pointer enters a world cell.
    MouseEnterCell,
    /// Fired when the pointer leaves a world cell.
    MouseLeaveCell,
    /// Fired when a key is pressed.
    KeyPressed,
    /// Fired when a key is released.
    KeyReleased,
    /// Fired when the active overlay changes.
    OverlayChanged,
    /// Fired when the engine asks plugins to contribute HUD lines for one cell.
    BuildHudForCell,
    /// Fired when a plugin-controlled overlay should submit one frame.
    RenderOverlay,
}

impl PluginEvent {
    /// Converts this event kind into the stable ABI tag.
    pub fn abi_kind(self) -> FluxEventKind {
        match self {
            Self::WorldCreated => FluxEventKind::WorldCreated,
            Self::WorldLoaded => FluxEventKind::WorldLoaded,
            Self::WorldBeforeSave => FluxEventKind::WorldBeforeSave,
            Self::WorldAfterSave => FluxEventKind::WorldAfterSave,
            Self::WorldUnloaded => FluxEventKind::WorldUnloaded,
            Self::SimulationPreCellGasStep => FluxEventKind::SimulationPreCellGasStep,
            Self::SimulationPostCellGasStep => FluxEventKind::SimulationPostCellGasStep,
            Self::SimulationPausedChanged => FluxEventKind::SimulationPausedChanged,
            Self::EntityPlaced => FluxEventKind::EntityPlaced,
            Self::EntityRemoved => FluxEventKind::EntityRemoved,
            Self::ToolSelected => FluxEventKind::ToolSelected,
            Self::MouseDownCell => FluxEventKind::MouseDownCell,
            Self::MouseMoveCell => FluxEventKind::MouseMoveCell,
            Self::MouseUpCell => FluxEventKind::MouseUpCell,
            Self::MouseEnterCell => FluxEventKind::MouseEnterCell,
            Self::MouseLeaveCell => FluxEventKind::MouseLeaveCell,
            Self::KeyPressed => FluxEventKind::KeyPressed,
            Self::KeyReleased => FluxEventKind::KeyReleased,
            Self::OverlayChanged => FluxEventKind::OverlayChanged,
            Self::BuildHudForCell => FluxEventKind::BuildHudForCell,
            Self::RenderOverlay => FluxEventKind::RenderOverlay,
        }
    }

    /// Builds one event kind from the raw ABI tag.
    pub fn from_raw(raw: u32) -> Option<Self> {
        Some(match FluxEventKind::from_raw(raw)? {
            FluxEventKind::WorldCreated => Self::WorldCreated,
            FluxEventKind::WorldLoaded => Self::WorldLoaded,
            FluxEventKind::WorldBeforeSave => Self::WorldBeforeSave,
            FluxEventKind::WorldAfterSave => Self::WorldAfterSave,
            FluxEventKind::WorldUnloaded => Self::WorldUnloaded,
            FluxEventKind::SimulationPreCellGasStep => Self::SimulationPreCellGasStep,
            FluxEventKind::SimulationPostCellGasStep => Self::SimulationPostCellGasStep,
            FluxEventKind::SimulationPausedChanged => Self::SimulationPausedChanged,
            FluxEventKind::EntityPlaced => Self::EntityPlaced,
            FluxEventKind::EntityRemoved => Self::EntityRemoved,
            FluxEventKind::ToolSelected => Self::ToolSelected,
            FluxEventKind::MouseDownCell => Self::MouseDownCell,
            FluxEventKind::MouseMoveCell => Self::MouseMoveCell,
            FluxEventKind::MouseUpCell => Self::MouseUpCell,
            FluxEventKind::MouseEnterCell => Self::MouseEnterCell,
            FluxEventKind::MouseLeaveCell => Self::MouseLeaveCell,
            FluxEventKind::KeyPressed => Self::KeyPressed,
            FluxEventKind::KeyReleased => Self::KeyReleased,
            FluxEventKind::OverlayChanged => Self::OverlayChanged,
            FluxEventKind::BuildHudForCell => Self::BuildHudForCell,
            FluxEventKind::BuildPanel => return None,
            FluxEventKind::RenderOverlay => Self::RenderOverlay,
        })
    }
}

/// Trait implemented by every typed event payload.
pub trait AbiEventPayload: Sized + 'static {
    /// Subscription kind that delivers this typed event.
    const KIND: PluginEvent;

    /// Decodes one typed event payload from the ABI dispatch bytes.
    unsafe fn decode(payload: *const u8, payload_len: usize) -> Result<Self, PluginError>;
}

/// Event fired after a new world has been created.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorldCreatedEvent;
/// Event fired after a world has been loaded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorldLoadedEvent;
/// Event fired right before the engine saves the current world.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorldBeforeSaveEvent;
/// Event fired after the engine has saved the current world.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorldAfterSaveEvent;
/// Event fired before the current world is unloaded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorldUnloadedEvent;
/// Event fired before one gas simulation step starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SimulationPreCellGasStepEvent;
/// Event fired after one gas simulation step completes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SimulationPostCellGasStepEvent;

/// Event fired when pause state changes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SimulationPausedChangedEvent {
    pub paused: bool,
}

/// Shared payload for entity placement and removal events.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntityEvent {
    pub id: EntityInstanceId,
    pub kind: EntityKindId,
    pub cell: UVec2,
}

/// Shared payload for mouse events that target one world cell.
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

/// Shared payload for key press and release events.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    pub key: String,
    pub modifiers: InputModifiers,
}

/// Event fired when the active tool changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolSelectedEvent {
    pub tool_id: Option<ContentId>,
}

/// Event fired when the active overlay changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlayChangedEvent {
    pub overlay_id: Option<OverlayModeId>,
}

/// Event fired when the engine asks plugins to build HUD lines for one cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuildHudForCellEvent {
    pub cell: UVec2,
}

/// Event fired when a plugin-controlled overlay should submit one frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderOverlayEvent {
    pub overlay_id: OverlayModeId,
}

macro_rules! impl_empty_event {
    ($ty:ty, $kind:expr) => {
        impl AbiEventPayload for $ty {
            const KIND: PluginEvent = $kind;

            unsafe fn decode(payload: *const u8, payload_len: usize) -> Result<Self, PluginError> {
                validate_payload::<FluxEmptyEventPayload>(payload, payload_len)?;
                Ok(Self)
            }
        }
    };
}

impl_empty_event!(WorldCreatedEvent, PluginEvent::WorldCreated);
impl_empty_event!(WorldLoadedEvent, PluginEvent::WorldLoaded);
impl_empty_event!(WorldBeforeSaveEvent, PluginEvent::WorldBeforeSave);
impl_empty_event!(WorldAfterSaveEvent, PluginEvent::WorldAfterSave);
impl_empty_event!(WorldUnloadedEvent, PluginEvent::WorldUnloaded);
impl_empty_event!(SimulationPreCellGasStepEvent, PluginEvent::SimulationPreCellGasStep);
impl_empty_event!(SimulationPostCellGasStepEvent, PluginEvent::SimulationPostCellGasStep);

impl AbiEventPayload for SimulationPausedChangedEvent {
    const KIND: PluginEvent = PluginEvent::SimulationPausedChanged;

    unsafe fn decode(payload: *const u8, payload_len: usize) -> Result<Self, PluginError> {
        let payload = validate_payload::<FluxSimulationPausedChangedEvent>(payload, payload_len)?;
        Ok(Self {
            paused: payload.paused != 0,
        })
    }
}

impl AbiEventPayload for EntityEvent {
    const KIND: PluginEvent = PluginEvent::EntityPlaced;

    unsafe fn decode(payload: *const u8, payload_len: usize) -> Result<Self, PluginError> {
        let payload = validate_payload::<FluxEntityEventPayload>(payload, payload_len)?;
        Ok(Self {
            id: EntityInstanceId(payload.entity_id),
            kind: ContentId::parse(&read_utf8(payload.entity_kind)?)?,
            cell: UVec2::new(payload.cell_x, payload.cell_y),
        })
    }
}

impl AbiEventPayload for MouseCellEvent {
    const KIND: PluginEvent = PluginEvent::MouseDownCell;

    unsafe fn decode(payload: *const u8, payload_len: usize) -> Result<Self, PluginError> {
        let payload = validate_payload::<FluxMouseCellEventPayload>(payload, payload_len)?;
        Ok(Self {
            button: decode_mouse_button(payload.button),
            cell: UVec2::new(payload.cell_x, payload.cell_y),
            world_position: Vec2::new(payload.world_x, payload.world_y),
            screen_position: Vec2::new(payload.screen_x, payload.screen_y),
            modifiers: decode_modifiers(payload.modifiers),
            active_tool_id: if payload.has_active_tool_id == 0 {
                None
            } else {
                Some(ContentId::parse(&read_utf8(payload.active_tool_id)?)?)
            },
            is_over_ui: payload.is_over_ui != 0,
        })
    }
}

impl AbiEventPayload for KeyEvent {
    const KIND: PluginEvent = PluginEvent::KeyPressed;

    unsafe fn decode(payload: *const u8, payload_len: usize) -> Result<Self, PluginError> {
        let payload = validate_payload::<FluxKeyEventPayload>(payload, payload_len)?;
        Ok(Self {
            key: read_utf8(payload.key)?,
            modifiers: decode_modifiers(payload.modifiers),
        })
    }
}

impl AbiEventPayload for ToolSelectedEvent {
    const KIND: PluginEvent = PluginEvent::ToolSelected;

    unsafe fn decode(payload: *const u8, payload_len: usize) -> Result<Self, PluginError> {
        let payload = validate_payload::<FluxToolSelectedEventPayload>(payload, payload_len)?;
        Ok(Self {
            tool_id: if payload.has_tool_id == 0 {
                None
            } else {
                Some(ContentId::parse(&read_utf8(payload.tool_id)?)?)
            },
        })
    }
}

impl AbiEventPayload for OverlayChangedEvent {
    const KIND: PluginEvent = PluginEvent::OverlayChanged;

    unsafe fn decode(payload: *const u8, payload_len: usize) -> Result<Self, PluginError> {
        let payload = validate_payload::<FluxOverlayChangedEventPayload>(payload, payload_len)?;
        Ok(Self {
            overlay_id: if payload.has_overlay_id == 0 {
                None
            } else {
                Some(ContentId::parse(&read_utf8(payload.overlay_id)?)?)
            },
        })
    }
}

impl AbiEventPayload for BuildHudForCellEvent {
    const KIND: PluginEvent = PluginEvent::BuildHudForCell;

    unsafe fn decode(payload: *const u8, payload_len: usize) -> Result<Self, PluginError> {
        let payload = validate_payload::<FluxBuildHudForCellEventPayload>(payload, payload_len)?;
        Ok(Self {
            cell: UVec2::new(payload.cell_x, payload.cell_y),
        })
    }
}

impl AbiEventPayload for RenderOverlayEvent {
    const KIND: PluginEvent = PluginEvent::RenderOverlay;

    unsafe fn decode(payload: *const u8, payload_len: usize) -> Result<Self, PluginError> {
        let payload = validate_payload::<FluxRenderOverlayEventPayload>(payload, payload_len)?;
        Ok(Self {
            overlay_id: ContentId::parse(&read_utf8(payload.overlay_id)?)?,
        })
    }
}

unsafe fn validate_payload<T>(payload: *const u8, payload_len: usize) -> Result<&'static T, PluginError> {
    if payload.is_null() || payload_len < std::mem::size_of::<T>() {
        return Err(PluginError::InvalidArgument("invalid ABI payload".to_string()));
    }
    Ok(&*(payload.cast::<T>()))
}

fn read_utf8(slice: FluxUtf8Slice) -> Result<String, PluginError> {
    if slice.len == 0 {
        return Ok(String::new());
    }
    if slice.ptr.is_null() {
        return Err(PluginError::InvalidArgument("null UTF-8 pointer".to_string()));
    }
    let bytes = unsafe { std::slice::from_raw_parts(slice.ptr, slice.len) };
    std::str::from_utf8(bytes)
        .map(str::to_string)
        .map_err(|error| PluginError::InvalidArgument(format!("invalid UTF-8: {error}")))
}

fn decode_mouse_button(value: u32) -> Option<MouseButton> {
    Some(match value {
        1 => MouseButton::Left,
        2 => MouseButton::Right,
        3 => MouseButton::Middle,
        0 => return None,
        other => MouseButton::Other(other as u16),
    })
}

fn decode_modifiers(flags: u32) -> InputModifiers {
    InputModifiers {
        shift: flags & 0b001 != 0,
        ctrl: flags & 0b010 != 0,
        alt: flags & 0b100 != 0,
    }
}
