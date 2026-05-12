use bevy_math::{UVec2, Vec2};

use crate::ENGINE_PLUGIN_API_VERSION_VALUE;

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
    EntityPlaced = 8,
    EntityRemoved = 9,
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
            8 => Self::EntityPlaced,
            9 => Self::EntityRemoved,
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
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxEmptyEventPayload {
    pub struct_size: u32,
    pub api_version: u32,
}

impl FluxEmptyEventPayload {
    pub fn new() -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>() as u32,
            api_version: ENGINE_PLUGIN_API_VERSION_VALUE,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxSimulationPausedChangedEvent {
    pub struct_size: u32,
    pub api_version: u32,
    pub paused: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxEntityEventPayload {
    pub struct_size: u32,
    pub api_version: u32,
    pub entity_id: u32,
    pub entity_kind: super::FluxUtf8Slice,
    pub cell_x: u32,
    pub cell_y: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxMouseCellEventPayload {
    pub struct_size: u32,
    pub api_version: u32,
    pub button: u32,
    pub cell_x: u32,
    pub cell_y: u32,
    pub world_x: f32,
    pub world_y: f32,
    pub screen_x: f32,
    pub screen_y: f32,
    pub modifiers: u32,
    pub has_active_tool_id: u8,
    pub active_tool_id: super::FluxUtf8Slice,
    pub is_over_ui: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxKeyEventPayload {
    pub struct_size: u32,
    pub api_version: u32,
    pub key: super::FluxUtf8Slice,
    pub modifiers: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxToolSelectedEventPayload {
    pub struct_size: u32,
    pub api_version: u32,
    pub has_tool_id: u8,
    pub tool_id: super::FluxUtf8Slice,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxOverlayChangedEventPayload {
    pub struct_size: u32,
    pub api_version: u32,
    pub has_overlay_id: u8,
    pub overlay_id: super::FluxUtf8Slice,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxBuildHudForCellEventPayload {
    pub struct_size: u32,
    pub api_version: u32,
    pub cell_x: u32,
    pub cell_y: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxBuildPanelEventPayload {
    pub struct_size: u32,
    pub api_version: u32,
    pub panel_id: super::FluxUtf8Slice,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxRenderOverlayEventPayload {
    pub struct_size: u32,
    pub api_version: u32,
    pub overlay_id: super::FluxUtf8Slice,
}

pub fn encode_uvec2(cell: UVec2) -> (u32, u32) {
    (cell.x, cell.y)
}

pub fn encode_vec2(value: Vec2) -> (f32, f32) {
    (value.x, value.y)
}
