use crate::{
    plugins::{
        abi::{
            FluxBuildHudForCellEventPayload, FluxBuildPanelEventPayload, FluxEmptyEventPayload,
            FluxEntityEventPayload, FluxKeyEventPayload, FluxMouseCellEventPayload,
            FluxOverlayChangedEventPayload, FluxPluginDispatchFn, FluxPluginHandle,
            FluxRenderOverlayEventPayload, FluxRuntimeHost, FluxSimulationPausedChangedEvent,
            FluxToolSelectedEventPayload, FluxUtf8Slice,
        },
        api::events::{InputModifiers, MouseButton, MouseCellEvent, PluginRuntimeEvent},
    },
    world::structures::PlacedStructureId,
};

use super::RuntimeHostContext;

pub(super) fn dispatch_to_plugin(
    handle: *mut FluxPluginHandle,
    dispatch_fn: FluxPluginDispatchFn,
    event: &PluginRuntimeEvent,
    context: &mut RuntimeHostContext,
) {
    let mut host = FluxRuntimeHost {
        struct_size: std::mem::size_of::<FluxRuntimeHost>() as u32,
        api_version: crate::plugins::ENGINE_PLUGIN_API_VERSION_VALUE,
        context: (context as *mut RuntimeHostContext).cast(),
        entity_place_fn: Some(super::entity_place_callback),
        entity_remove_fn: Some(super::entity_remove_callback),
        entity_remove_at_fn: Some(super::entity_remove_at_callback),
        entity_set_rotation_fn: Some(super::entity_set_rotation_callback),
        entity_set_enabled_fn: Some(super::entity_set_enabled_callback),
        entity_set_label_fn: Some(super::entity_set_label_callback),
        gas_add_fn: Some(super::add_gas_callback),
        gas_remove_fn: Some(super::remove_gas_callback),
        gas_clear_cell_fn: Some(super::clear_gas_cell_callback),
        gas_pressure_at_fn: Some(super::gas_pressure_at_callback),
        gas_amount_at_fn: Some(super::gas_amount_at_callback),
        submit_hud_block_fn: Some(super::submit_hud_block_callback),
        submit_overlay_frame_fn: Some(super::submit_overlay_frame_callback),
        write_save_chunk_fn: Some(super::write_save_chunk_callback),
        read_save_chunk_fn: Some(super::read_save_chunk_callback),
        delete_save_chunk_fn: Some(super::delete_save_chunk_callback),
        get_time_snapshot_fn: Some(super::get_time_snapshot_callback),
        set_paused_fn: Some(super::set_paused_callback),
        toggle_pause_fn: Some(super::toggle_pause_callback),
        set_speed_fn: Some(super::set_speed_callback),
        set_active_tool_fn: Some(super::set_active_tool_callback),
        write_log_fn: Some(super::write_runtime_log_callback),
    };

    unsafe {
        match event {
            PluginRuntimeEvent::WorldCreated => {
                let payload = FluxEmptyEventPayload::new();
                let _ = dispatch_fn(handle, 0, (&payload as *const FluxEmptyEventPayload).cast(), std::mem::size_of_val(&payload), &mut host);
            }
            PluginRuntimeEvent::WorldLoaded => {
                let payload = FluxEmptyEventPayload::new();
                let _ = dispatch_fn(handle, 1, (&payload as *const FluxEmptyEventPayload).cast(), std::mem::size_of_val(&payload), &mut host);
            }
            PluginRuntimeEvent::WorldBeforeSave => {
                let payload = FluxEmptyEventPayload::new();
                let _ = dispatch_fn(handle, 2, (&payload as *const FluxEmptyEventPayload).cast(), std::mem::size_of_val(&payload), &mut host);
            }
            PluginRuntimeEvent::WorldAfterSave => {
                let payload = FluxEmptyEventPayload::new();
                let _ = dispatch_fn(handle, 3, (&payload as *const FluxEmptyEventPayload).cast(), std::mem::size_of_val(&payload), &mut host);
            }
            PluginRuntimeEvent::WorldUnloaded => {
                let payload = FluxEmptyEventPayload::new();
                let _ = dispatch_fn(handle, 4, (&payload as *const FluxEmptyEventPayload).cast(), std::mem::size_of_val(&payload), &mut host);
            }
            PluginRuntimeEvent::SimulationPreCellGasStep => {
                let payload = FluxEmptyEventPayload::new();
                let _ = dispatch_fn(handle, 5, (&payload as *const FluxEmptyEventPayload).cast(), std::mem::size_of_val(&payload), &mut host);
            }
            PluginRuntimeEvent::SimulationPostCellGasStep => {
                let payload = FluxEmptyEventPayload::new();
                let _ = dispatch_fn(handle, 6, (&payload as *const FluxEmptyEventPayload).cast(), std::mem::size_of_val(&payload), &mut host);
            }
            PluginRuntimeEvent::SimulationPausedChanged { paused } => {
                let payload = FluxSimulationPausedChangedEvent {
                    struct_size: std::mem::size_of::<FluxSimulationPausedChangedEvent>() as u32,
                    api_version: crate::plugins::ENGINE_PLUGIN_API_VERSION_VALUE,
                    paused: *paused as u8,
                };
                let _ = dispatch_fn(handle, 7, (&payload as *const FluxSimulationPausedChangedEvent).cast(), std::mem::size_of_val(&payload), &mut host);
            }
            PluginRuntimeEvent::StructurePlaced(event) => {
                let payload = build_entity_payload(event.id, event.kind.as_str(), event.cell);
                let _ = dispatch_fn(handle, 8, (&payload as *const FluxEntityEventPayload).cast(), std::mem::size_of_val(&payload), &mut host);
            }
            PluginRuntimeEvent::StructureRemoved(event) => {
                let payload = build_entity_payload(event.id, event.kind.as_str(), event.cell);
                let _ = dispatch_fn(handle, 9, (&payload as *const FluxEntityEventPayload).cast(), std::mem::size_of_val(&payload), &mut host);
            }
            PluginRuntimeEvent::ToolSelected { tool_id } => {
                let payload = FluxToolSelectedEventPayload {
                    struct_size: std::mem::size_of::<FluxToolSelectedEventPayload>() as u32,
                    api_version: crate::plugins::ENGINE_PLUGIN_API_VERSION_VALUE,
                    has_tool_id: tool_id.is_some() as u8,
                    tool_id: FluxUtf8Slice::from_str(tool_id.as_ref().map(|id| id.as_str()).unwrap_or("")),
                };
                let _ = dispatch_fn(handle, 10, (&payload as *const FluxToolSelectedEventPayload).cast(), std::mem::size_of_val(&payload), &mut host);
            }
            PluginRuntimeEvent::MouseDownCell(mouse) => dispatch_mouse(handle, dispatch_fn, 11, mouse, &mut host),
            PluginRuntimeEvent::MouseMoveCell(mouse) => dispatch_mouse(handle, dispatch_fn, 12, mouse, &mut host),
            PluginRuntimeEvent::MouseUpCell(mouse) => dispatch_mouse(handle, dispatch_fn, 13, mouse, &mut host),
            PluginRuntimeEvent::MouseEnterCell(mouse) => dispatch_mouse(handle, dispatch_fn, 14, mouse, &mut host),
            PluginRuntimeEvent::MouseLeaveCell(mouse) => dispatch_mouse(handle, dispatch_fn, 15, mouse, &mut host),
            PluginRuntimeEvent::KeyPressed { key, modifiers } => {
                let payload = build_key_payload(key, *modifiers);
                let _ = dispatch_fn(handle, 16, (&payload as *const FluxKeyEventPayload).cast(), std::mem::size_of_val(&payload), &mut host);
            }
            PluginRuntimeEvent::KeyReleased { key, modifiers } => {
                let payload = build_key_payload(key, *modifiers);
                let _ = dispatch_fn(handle, 17, (&payload as *const FluxKeyEventPayload).cast(), std::mem::size_of_val(&payload), &mut host);
            }
            PluginRuntimeEvent::OverlayChanged { overlay_id } => {
                let payload = FluxOverlayChangedEventPayload {
                    struct_size: std::mem::size_of::<FluxOverlayChangedEventPayload>() as u32,
                    api_version: crate::plugins::ENGINE_PLUGIN_API_VERSION_VALUE,
                    has_overlay_id: overlay_id.is_some() as u8,
                    overlay_id: FluxUtf8Slice::from_str(overlay_id.as_ref().map(|id| id.as_str()).unwrap_or("")),
                };
                let _ = dispatch_fn(handle, 18, (&payload as *const FluxOverlayChangedEventPayload).cast(), std::mem::size_of_val(&payload), &mut host);
            }
            PluginRuntimeEvent::BuildHudForCell { cell } => {
                let payload = FluxBuildHudForCellEventPayload {
                    struct_size: std::mem::size_of::<FluxBuildHudForCellEventPayload>() as u32,
                    api_version: crate::plugins::ENGINE_PLUGIN_API_VERSION_VALUE,
                    cell_x: cell.x,
                    cell_y: cell.y,
                };
                let _ = dispatch_fn(handle, 19, (&payload as *const FluxBuildHudForCellEventPayload).cast(), std::mem::size_of_val(&payload), &mut host);
            }
            PluginRuntimeEvent::BuildPanel { panel_id } => {
                let payload = FluxBuildPanelEventPayload {
                    struct_size: std::mem::size_of::<FluxBuildPanelEventPayload>() as u32,
                    api_version: crate::plugins::ENGINE_PLUGIN_API_VERSION_VALUE,
                    panel_id: FluxUtf8Slice::from_str(panel_id.as_str()),
                };
                let _ = dispatch_fn(handle, 20, (&payload as *const FluxBuildPanelEventPayload).cast(), std::mem::size_of_val(&payload), &mut host);
            }
            PluginRuntimeEvent::RenderOverlay { overlay_id } => {
                let payload = FluxRenderOverlayEventPayload {
                    struct_size: std::mem::size_of::<FluxRenderOverlayEventPayload>() as u32,
                    api_version: crate::plugins::ENGINE_PLUGIN_API_VERSION_VALUE,
                    overlay_id: FluxUtf8Slice::from_str(overlay_id.as_str()),
                };
                let _ = dispatch_fn(handle, 21, (&payload as *const FluxRenderOverlayEventPayload).cast(), std::mem::size_of_val(&payload), &mut host);
            }
        }
    }
}

unsafe fn dispatch_mouse(
    handle: *mut FluxPluginHandle,
    dispatch_fn: FluxPluginDispatchFn,
    event_kind: u32,
    mouse: &MouseCellEvent,
    host: &mut FluxRuntimeHost,
) {
    let payload = FluxMouseCellEventPayload {
        struct_size: std::mem::size_of::<FluxMouseCellEventPayload>() as u32,
        api_version: crate::plugins::ENGINE_PLUGIN_API_VERSION_VALUE,
        button: encode_mouse_button(mouse.button),
        cell_x: mouse.cell.x,
        cell_y: mouse.cell.y,
        world_x: mouse.world_position.x,
        world_y: mouse.world_position.y,
        screen_x: mouse.screen_position.x,
        screen_y: mouse.screen_position.y,
        modifiers: encode_modifiers(mouse.modifiers),
        has_active_tool_id: mouse.active_tool_id.is_some() as u8,
        active_tool_id: FluxUtf8Slice::from_str(mouse.active_tool_id.as_ref().map(|id| id.as_str()).unwrap_or("")),
        is_over_ui: mouse.is_over_ui as u8,
    };
    let _ = dispatch_fn(handle, event_kind, (&payload as *const FluxMouseCellEventPayload).cast(), std::mem::size_of_val(&payload), host);
}

fn build_entity_payload(
    structure_id: PlacedStructureId,
    entity_kind: &str,
    cell: bevy::prelude::UVec2,
) -> FluxEntityEventPayload {
    FluxEntityEventPayload {
        struct_size: std::mem::size_of::<FluxEntityEventPayload>() as u32,
        api_version: crate::plugins::ENGINE_PLUGIN_API_VERSION_VALUE,
        entity_id: structure_id.0,
        entity_kind: FluxUtf8Slice::from_str(entity_kind),
        cell_x: cell.x,
        cell_y: cell.y,
    }
}

fn build_key_payload(key: &str, modifiers: InputModifiers) -> FluxKeyEventPayload {
    FluxKeyEventPayload {
        struct_size: std::mem::size_of::<FluxKeyEventPayload>() as u32,
        api_version: crate::plugins::ENGINE_PLUGIN_API_VERSION_VALUE,
        key: FluxUtf8Slice::from_str(key),
        modifiers: encode_modifiers(modifiers),
    }
}

fn encode_mouse_button(button: Option<MouseButton>) -> u32 {
    match button {
        Some(MouseButton::Left) => 1,
        Some(MouseButton::Right) => 2,
        Some(MouseButton::Middle) => 3,
        Some(MouseButton::Other(value)) => value as u32,
        None => 0,
    }
}

fn encode_modifiers(modifiers: InputModifiers) -> u32 {
    (modifiers.shift as u32) | ((modifiers.ctrl as u32) << 1) | ((modifiers.alt as u32) << 2)
}
