use std::ffi::c_void;

use bevy::prelude::Vec2;
use flux_plugin_abi::{FluxStatus, FluxUtf8Slice};
use flux_plugin_sdk::{
    __private::{RuntimeHostBinding, RuntimeHostFns},
    PluginError,
};

use crate::plugins::runtime_dll::{
    add_gas_callback, clear_gas_cell_callback, delete_save_chunk_callback, entity_place_callback,
    entity_remove_at_callback, entity_remove_callback, entity_set_enabled_callback,
    entity_set_label_callback, entity_set_rotation_callback, gas_amount_at_callback,
    gas_pressure_at_callback, get_time_snapshot_callback, read_save_chunk_callback,
    remove_gas_callback, set_active_tool_callback, set_paused_callback, submit_hud_block_callback,
    submit_overlay_graph_callback, toggle_pause_callback, write_runtime_log_callback,
    write_save_chunk_callback, RuntimeHostContext,
};

/// Builds an SDK runtime-host binding backed by the engine `RuntimeHostContext`.
pub(crate) fn sdk_runtime_host_binding(context: &mut RuntimeHostContext) -> RuntimeHostBinding {
    RuntimeHostBinding::new(
        (context as *mut RuntimeHostContext).cast::<c_void>(),
        &SDK_RUNTIME_HOST_FNS,
    )
}

static SDK_RUNTIME_HOST_FNS: RuntimeHostFns = RuntimeHostFns {
    entity_place: sdk_entity_place,
    entity_remove: sdk_entity_remove,
    entity_remove_at: sdk_entity_remove_at,
    entity_set_rotation: sdk_entity_set_rotation,
    entity_set_enabled: sdk_entity_set_enabled,
    entity_set_label: sdk_entity_set_label,
    gas_add: sdk_gas_add,
    gas_remove: sdk_gas_remove,
    gas_clear_cell: sdk_gas_clear_cell,
    gas_pressure_at: sdk_gas_pressure_at,
    gas_amount_at: sdk_gas_amount_at,
    submit_hud_line: sdk_submit_hud_line,
    submit_overlay_graph: sdk_submit_overlay_graph,
    write_save_chunk: sdk_write_save_chunk,
    read_save_chunk: sdk_read_save_chunk,
    delete_save_chunk: sdk_delete_save_chunk,
    time_snapshot: sdk_time_snapshot,
    set_paused: sdk_set_paused,
    toggle_pause: sdk_toggle_pause,
    set_speed: sdk_set_speed,
    set_active_tool: sdk_set_active_tool,
    write_log: sdk_write_log,
};

fn sdk_entity_place(
    context: *mut c_void,
    kind_id: &str,
    origin_x: u32,
    origin_y: u32,
    rotation: u32,
) -> Result<u32, PluginError> {
    let mut entity_id = 0u32;
    unsafe {
        entity_place_callback(
            context,
            FluxUtf8Slice::from_str(kind_id),
            origin_x,
            origin_y,
            rotation,
            &mut entity_id,
        )
    }
    .into_result()
    .map_err(status_error)?;
    Ok(entity_id)
}

fn sdk_entity_remove(context: *mut c_void, entity_id: u32) -> Result<(), PluginError> {
    unsafe { entity_remove_callback(context, entity_id) }
        .into_result()
        .map_err(status_error)
}

fn sdk_entity_remove_at(
    context: *mut c_void,
    x: u32,
    y: u32,
    layer_id: &str,
) -> Result<Option<u32>, PluginError> {
    let mut removed = 0u32;
    unsafe {
        entity_remove_at_callback(
            context,
            x,
            y,
            FluxUtf8Slice::from_str(layer_id),
            &mut removed,
        )
    }
    .into_result()
    .map_err(status_error)?;
    Ok((removed != 0).then_some(removed))
}

fn sdk_entity_set_rotation(
    context: *mut c_void,
    entity_id: u32,
    rotation: u32,
) -> Result<(), PluginError> {
    unsafe { entity_set_rotation_callback(context, entity_id, rotation) }
        .into_result()
        .map_err(status_error)
}

fn sdk_entity_set_enabled(
    context: *mut c_void,
    entity_id: u32,
    enabled: bool,
) -> Result<(), PluginError> {
    unsafe { entity_set_enabled_callback(context, entity_id, enabled as u8) }
        .into_result()
        .map_err(status_error)
}

fn sdk_entity_set_label(
    context: *mut c_void,
    entity_id: u32,
    label: &str,
) -> Result<(), PluginError> {
    unsafe { entity_set_label_callback(context, entity_id, FluxUtf8Slice::from_str(label)) }
        .into_result()
        .map_err(status_error)
}

fn sdk_gas_add(
    context: *mut c_void,
    x: u32,
    y: u32,
    substance: &str,
    amount: u32,
    velocity: Vec2,
) -> Result<u32, PluginError> {
    let mut added = 0u32;
    unsafe {
        add_gas_callback(
            context,
            x,
            y,
            FluxUtf8Slice::from_str(substance),
            amount,
            velocity.x,
            velocity.y,
            &mut added,
        )
    }
    .into_result()
    .map_err(status_error)?;
    Ok(added)
}

fn sdk_gas_remove(
    context: *mut c_void,
    x: u32,
    y: u32,
    substance: &str,
    amount: u32,
) -> Result<u32, PluginError> {
    let mut removed = 0u32;
    unsafe {
        remove_gas_callback(
            context,
            x,
            y,
            FluxUtf8Slice::from_str(substance),
            amount,
            &mut removed,
        )
    }
    .into_result()
    .map_err(status_error)?;
    Ok(removed)
}

fn sdk_gas_clear_cell(context: *mut c_void, x: u32, y: u32) -> Result<(), PluginError> {
    unsafe { clear_gas_cell_callback(context, x, y) }
        .into_result()
        .map_err(status_error)
}

fn sdk_gas_pressure_at(context: *mut c_void, x: u32, y: u32) -> Result<f32, PluginError> {
    let mut pressure = 0.0f32;
    unsafe { gas_pressure_at_callback(context, x, y, &mut pressure) }
        .into_result()
        .map_err(status_error)?;
    Ok(pressure)
}

fn sdk_gas_amount_at(
    context: *mut c_void,
    x: u32,
    y: u32,
    substance: &str,
) -> Result<u32, PluginError> {
    let mut amount = 0u32;
    unsafe {
        gas_amount_at_callback(
            context,
            x,
            y,
            FluxUtf8Slice::from_str(substance),
            &mut amount,
        )
    }
    .into_result()
    .map_err(status_error)?;
    Ok(amount)
}

fn sdk_submit_hud_line(
    context: *mut c_void,
    block_id: &str,
    title: &str,
    line: &str,
    sort_order: i32,
) -> Result<(), PluginError> {
    unsafe {
        submit_hud_block_callback(
            context,
            FluxUtf8Slice::from_str(block_id),
            FluxUtf8Slice::from_str(title),
            FluxUtf8Slice::from_str(line),
            sort_order,
        )
    }
    .into_result()
    .map_err(status_error)
}

fn sdk_submit_overlay_graph(
    context: *mut c_void,
    graph: &flux_plugin_sdk::OverlayGraph,
) -> Result<(), PluginError> {
    let json = serde_json::to_string(graph).map_err(|error| {
        PluginError::message(format!("failed to encode overlay graph: {error}"))
    })?;
    unsafe { submit_overlay_graph_callback(context, FluxUtf8Slice::from_str(&json)) }
        .into_result()
        .map_err(status_error)
}

fn sdk_write_save_chunk(
    context: *mut c_void,
    chunk_id: &str,
    version: u32,
    bytes: &[u8],
) -> Result<(), PluginError> {
    unsafe {
        write_save_chunk_callback(
            context,
            FluxUtf8Slice::from_str(chunk_id),
            version,
            bytes.as_ptr(),
            bytes.len(),
        )
    }
    .into_result()
    .map_err(status_error)
}

fn sdk_read_save_chunk(
    context: *mut c_void,
    chunk_id: &str,
) -> Result<Option<flux_plugin_sdk::__private::RuntimeSaveChunkData>, PluginError> {
    let mut version = 0u32;
    let mut len = 0usize;
    let status = unsafe {
        read_save_chunk_callback(
            context,
            FluxUtf8Slice::from_str(chunk_id),
            &mut version,
            std::ptr::null_mut(),
            0,
            &mut len,
        )
    };
    if status == FluxStatus::FAILED {
        return Ok(None);
    }
    status.into_result().map_err(status_error)?;
    let mut bytes = vec![0u8; len];
    unsafe {
        read_save_chunk_callback(
            context,
            FluxUtf8Slice::from_str(chunk_id),
            &mut version,
            bytes.as_mut_ptr(),
            bytes.len(),
            &mut len,
        )
    }
    .into_result()
    .map_err(status_error)?;
    bytes.truncate(len);
    Ok(Some(flux_plugin_sdk::__private::RuntimeSaveChunkData {
        version,
        bytes,
    }))
}

fn sdk_delete_save_chunk(context: *mut c_void, chunk_id: &str) -> Result<(), PluginError> {
    unsafe { delete_save_chunk_callback(context, FluxUtf8Slice::from_str(chunk_id)) }
        .into_result()
        .map_err(status_error)
}

fn sdk_time_snapshot(
    context: *mut c_void,
) -> Result<flux_plugin_sdk::__private::RuntimeTimeSnapshot, PluginError> {
    let mut snapshot = flux_plugin_abi::FluxTimeSnapshot::default();
    unsafe { get_time_snapshot_callback(context, &mut snapshot) }
        .into_result()
        .map_err(status_error)?;
    Ok(flux_plugin_sdk::__private::RuntimeTimeSnapshot {
        tick: snapshot.tick,
        paused: snapshot.paused != 0,
        speed: snapshot.speed,
        delta_seconds: snapshot.delta_seconds,
    })
}

fn sdk_set_paused(context: *mut c_void, paused: bool) -> Result<(), PluginError> {
    unsafe { set_paused_callback(context, paused as u8) }
        .into_result()
        .map_err(status_error)
}

fn sdk_toggle_pause(context: *mut c_void) -> Result<bool, PluginError> {
    let mut paused = 0u8;
    unsafe { toggle_pause_callback(context, &mut paused) }
        .into_result()
        .map_err(status_error)?;
    Ok(paused != 0)
}

fn sdk_set_speed(context: *mut c_void, speed: u32) -> Result<(), PluginError> {
    unsafe { crate::plugins::runtime_dll::set_speed_callback(context, speed) }
        .into_result()
        .map_err(status_error)
}

fn sdk_set_active_tool(context: *mut c_void, tool_id: Option<&str>) -> Result<(), PluginError> {
    unsafe { set_active_tool_callback(context, FluxUtf8Slice::from_str(tool_id.unwrap_or(""))) }
        .into_result()
        .map_err(status_error)
}

fn sdk_write_log(context: *mut c_void, level: u32, message: &str) -> Result<(), PluginError> {
    unsafe { write_runtime_log_callback(context, level, FluxUtf8Slice::from_str(message)) }
        .into_result()
        .map_err(status_error)
}

fn status_error(status: FluxStatus) -> PluginError {
    match status {
        FluxStatus::INVALID_ARGUMENT => {
            PluginError::InvalidArgument("host rejected arguments".to_string())
        }
        FluxStatus::API_UNAVAILABLE => PluginError::ApiUnavailable("runtime"),
        FluxStatus::UNSUPPORTED => PluginError::Unsupported("runtime"),
        _ => PluginError::message(format!("host call failed with status {}", status.code())),
    }
}
