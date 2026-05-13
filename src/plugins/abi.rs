use std::ffi::c_void;

use crate::plugins::api::events::PluginEvent;

pub use flux_plugin_abi::*;

/// Builds one host API payload for runtime plugin creation.
pub fn build_host_api(
    plugin_id: FluxUtf8Slice,
    engine_version: FluxUtf8Slice,
    plugin_root: FluxUtf8Slice,
    config_root: FluxUtf8Slice,
    assets_root: FluxUtf8Slice,
    write_log_fn: Option<FluxWriteLogFn>,
    log_context: *mut c_void,
) -> FluxHostApi {
    FluxHostApi {
        struct_size: std::mem::size_of::<FluxHostApi>() as u32,
        api_version: crate::plugins::id::ENGINE_PLUGIN_API_VERSION_VALUE,
        plugin_id,
        engine_version,
        plugin_root,
        config_root,
        assets_root,
        write_log_fn,
        log_context,
    }
}

/// Builds one registration callback table for the loader handshake.
#[allow(clippy::too_many_arguments)]
pub fn build_registrar(
    register_gas_substance_fn: Option<unsafe extern "C" fn(*mut c_void, *const FluxGasSubstanceDescriptor) -> FluxStatus>,
    register_entity_fn: Option<unsafe extern "C" fn(*mut c_void, *const FluxEntityDescriptor) -> FluxStatus>,
    register_tool_fn: Option<unsafe extern "C" fn(*mut c_void, *const FluxToolDescriptor) -> FluxStatus>,
    register_panel_fn: Option<unsafe extern "C" fn(*mut c_void, *const FluxPanelDescriptor) -> FluxStatus>,
    register_overlay_fn: Option<unsafe extern "C" fn(*mut c_void, *const FluxOverlayDescriptor) -> FluxStatus>,
    register_overlay_material_fn: Option<unsafe extern "C" fn(*mut c_void, *const FluxOverlayMaterialDescriptor) -> FluxStatus>,
    register_save_chunk_fn: Option<unsafe extern "C" fn(*mut c_void, *const FluxSaveChunkDescriptor) -> FluxStatus>,
    register_subscription_fn: Option<unsafe extern "C" fn(*mut c_void, *const FluxSubscriptionDescriptor) -> FluxStatus>,
    registration_context: *mut c_void,
) -> FluxRegistrar {
    FluxRegistrar {
        struct_size: std::mem::size_of::<FluxRegistrar>() as u32,
        api_version: crate::plugins::id::ENGINE_PLUGIN_API_VERSION_VALUE,
        register_gas_substance_fn,
        register_entity_fn,
        register_tool_fn,
        register_panel_fn,
        register_overlay_fn,
        register_overlay_material_fn,
        register_save_chunk_fn,
        register_subscription_fn,
        registration_context,
    }
}

/// Maps one stable ABI event tag into the engine runtime event category.
pub fn event_kind_from_abi(raw: u32) -> Option<PluginEvent> {
    Some(match FluxEventKind::from_raw(raw)? {
        FluxEventKind::WorldCreated => PluginEvent::WorldCreated,
        FluxEventKind::WorldLoaded => PluginEvent::WorldLoaded,
        FluxEventKind::WorldBeforeSave => PluginEvent::WorldBeforeSave,
        FluxEventKind::WorldAfterSave => PluginEvent::WorldAfterSave,
        FluxEventKind::WorldUnloaded => PluginEvent::WorldUnloaded,
        FluxEventKind::SimulationPreCellGasStep => PluginEvent::SimulationPreCellGasStep,
        FluxEventKind::SimulationPostCellGasStep => PluginEvent::SimulationPostCellGasStep,
        FluxEventKind::SimulationPausedChanged => PluginEvent::SimulationPausedChanged,
        FluxEventKind::EntityPlaced => PluginEvent::StructurePlaced,
        FluxEventKind::EntityRemoved => PluginEvent::StructureRemoved,
        FluxEventKind::ToolSelected => PluginEvent::ToolSelected,
        FluxEventKind::MouseDownCell => PluginEvent::MouseDownCell,
        FluxEventKind::MouseMoveCell => PluginEvent::MouseMoveCell,
        FluxEventKind::MouseUpCell => PluginEvent::MouseUpCell,
        FluxEventKind::MouseEnterCell => PluginEvent::MouseEnterCell,
        FluxEventKind::MouseLeaveCell => PluginEvent::MouseLeaveCell,
        FluxEventKind::KeyPressed => PluginEvent::KeyPressed,
        FluxEventKind::KeyReleased => PluginEvent::KeyReleased,
        FluxEventKind::OverlayChanged => PluginEvent::OverlayChanged,
        FluxEventKind::BuildHudForCell => PluginEvent::BuildHudForCell,
        FluxEventKind::BuildPanel => return None,
        FluxEventKind::RenderOverlay => PluginEvent::RenderOverlay,
    })
}
