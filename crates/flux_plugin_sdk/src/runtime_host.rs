use std::ffi::c_void;

use bevy_math::Vec2;
use flux_plugin_abi::{FluxRuntimeHost, FluxStatus, FluxTimeSnapshot, FluxUtf8Slice};

use crate::{ContentId, PluginError};

/// Internal runtime save-chunk snapshot returned by host adapters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeSaveChunkData {
    pub version: u32,
    pub bytes: Vec<u8>,
}

/// Internal simulation-time snapshot returned by host adapters.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RuntimeTimeSnapshot {
    pub tick: u64,
    pub paused: bool,
    pub speed: u32,
    pub delta_seconds: f32,
}

/// Internal function table used by SDK proxy APIs.
pub struct RuntimeHostFns {
    pub entity_place: fn(*mut c_void, &str, u32, u32, u32) -> Result<u32, PluginError>,
    pub entity_remove: fn(*mut c_void, u32) -> Result<(), PluginError>,
    pub entity_remove_at:
        fn(*mut c_void, u32, u32, &str) -> Result<Option<u32>, PluginError>,
    pub entity_set_rotation: fn(*mut c_void, u32, u32) -> Result<(), PluginError>,
    pub entity_set_enabled: fn(*mut c_void, u32, bool) -> Result<(), PluginError>,
    pub entity_set_label: fn(*mut c_void, u32, &str) -> Result<(), PluginError>,
    pub gas_add:
        fn(*mut c_void, u32, u32, &str, u32, Vec2) -> Result<u32, PluginError>,
    pub gas_remove: fn(*mut c_void, u32, u32, &str, u32) -> Result<u32, PluginError>,
    pub gas_clear_cell: fn(*mut c_void, u32, u32) -> Result<(), PluginError>,
    pub gas_pressure_at: fn(*mut c_void, u32, u32) -> Result<f32, PluginError>,
    pub gas_amount_at: fn(*mut c_void, u32, u32, &str) -> Result<u32, PluginError>,
    pub submit_hud_line:
        fn(*mut c_void, &str, &str, &str, i32) -> Result<(), PluginError>,
    pub submit_overlay_frame:
        fn(*mut c_void, u32, u32, &[u8]) -> Result<(), PluginError>,
    pub write_save_chunk:
        fn(*mut c_void, &str, u32, &[u8]) -> Result<(), PluginError>,
    pub read_save_chunk:
        fn(*mut c_void, &str) -> Result<Option<RuntimeSaveChunkData>, PluginError>,
    pub delete_save_chunk: fn(*mut c_void, &str) -> Result<(), PluginError>,
    pub time_snapshot: fn(*mut c_void) -> Result<RuntimeTimeSnapshot, PluginError>,
    pub set_paused: fn(*mut c_void, bool) -> Result<(), PluginError>,
    pub toggle_pause: fn(*mut c_void) -> Result<bool, PluginError>,
    pub set_speed: fn(*mut c_void, u32) -> Result<(), PluginError>,
    pub set_active_tool: fn(*mut c_void, Option<&str>) -> Result<(), PluginError>,
    pub write_log: fn(*mut c_void, u32, &str) -> Result<(), PluginError>,
}

/// One bound runtime host visible inside the current handler scope.
#[derive(Clone, Copy)]
pub struct RuntimeHostBinding {
    context: *mut c_void,
    fns: &'static RuntimeHostFns,
}

impl RuntimeHostBinding {
    /// Builds one bound runtime host from raw parts.
    pub const fn new(context: *mut c_void, fns: &'static RuntimeHostFns) -> Self {
        Self { context, fns }
    }

    /// Places one entity through the active host implementation.
    pub(crate) fn entity_place(
        &self,
        kind_id: &str,
        origin_x: u32,
        origin_y: u32,
        rotation: u32,
    ) -> Result<u32, PluginError> {
        (self.fns.entity_place)(self.context, kind_id, origin_x, origin_y, rotation)
    }

    /// Removes one entity through the active host implementation.
    pub(crate) fn entity_remove(&self, entity_id: u32) -> Result<(), PluginError> {
        (self.fns.entity_remove)(self.context, entity_id)
    }

    /// Removes one entity at a cell/layer pair.
    pub(crate) fn entity_remove_at(
        &self,
        x: u32,
        y: u32,
        layer_id: &str,
    ) -> Result<Option<u32>, PluginError> {
        (self.fns.entity_remove_at)(self.context, x, y, layer_id)
    }

    /// Updates one entity rotation.
    pub(crate) fn entity_set_rotation(
        &self,
        entity_id: u32,
        rotation: u32,
    ) -> Result<(), PluginError> {
        (self.fns.entity_set_rotation)(self.context, entity_id, rotation)
    }

    /// Enables or disables one entity.
    pub(crate) fn entity_set_enabled(
        &self,
        entity_id: u32,
        enabled: bool,
    ) -> Result<(), PluginError> {
        (self.fns.entity_set_enabled)(self.context, entity_id, enabled)
    }

    /// Sets one entity label.
    pub(crate) fn entity_set_label(
        &self,
        entity_id: u32,
        label: &str,
    ) -> Result<(), PluginError> {
        (self.fns.entity_set_label)(self.context, entity_id, label)
    }

    /// Adds gas through the active host implementation.
    pub(crate) fn gas_add(
        &self,
        x: u32,
        y: u32,
        substance: &str,
        amount: u32,
        velocity: Vec2,
    ) -> Result<u32, PluginError> {
        (self.fns.gas_add)(self.context, x, y, substance, amount, velocity)
    }

    /// Removes gas through the active host implementation.
    pub(crate) fn gas_remove(
        &self,
        x: u32,
        y: u32,
        substance: &str,
        amount: u32,
    ) -> Result<u32, PluginError> {
        (self.fns.gas_remove)(self.context, x, y, substance, amount)
    }

    /// Clears all gas from one world cell.
    pub(crate) fn gas_clear_cell(&self, x: u32, y: u32) -> Result<(), PluginError> {
        (self.fns.gas_clear_cell)(self.context, x, y)
    }

    /// Returns pressure for one world cell.
    pub(crate) fn gas_pressure_at(&self, x: u32, y: u32) -> Result<f32, PluginError> {
        (self.fns.gas_pressure_at)(self.context, x, y)
    }

    /// Returns one substance amount for one world cell.
    pub(crate) fn gas_amount_at(
        &self,
        x: u32,
        y: u32,
        substance: &str,
    ) -> Result<u32, PluginError> {
        (self.fns.gas_amount_at)(self.context, x, y, substance)
    }

    /// Appends one HUD line.
    pub(crate) fn submit_hud_line(
        &self,
        block_id: &str,
        title: &str,
        line: &str,
        sort_order: i32,
    ) -> Result<(), PluginError> {
        (self.fns.submit_hud_line)(self.context, block_id, title, line, sort_order)
    }

    /// Submits one complete overlay frame.
    pub(crate) fn submit_overlay_frame(
        &self,
        width: u32,
        height: u32,
        rgba8: &[u8],
    ) -> Result<(), PluginError> {
        (self.fns.submit_overlay_frame)(self.context, width, height, rgba8)
    }

    /// Writes one plugin-owned save chunk.
    pub(crate) fn write_save_chunk(
        &self,
        chunk_id: &str,
        version: u32,
        bytes: &[u8],
    ) -> Result<(), PluginError> {
        (self.fns.write_save_chunk)(self.context, chunk_id, version, bytes)
    }

    /// Reads one plugin-owned save chunk.
    pub(crate) fn read_save_chunk(
        &self,
        chunk_id: &str,
    ) -> Result<Option<RuntimeSaveChunkData>, PluginError> {
        (self.fns.read_save_chunk)(self.context, chunk_id)
    }

    /// Deletes one plugin-owned save chunk.
    pub(crate) fn delete_save_chunk(&self, chunk_id: &str) -> Result<(), PluginError> {
        (self.fns.delete_save_chunk)(self.context, chunk_id)
    }

    /// Returns the current simulation-time snapshot.
    pub(crate) fn time_snapshot(&self) -> Result<RuntimeTimeSnapshot, PluginError> {
        (self.fns.time_snapshot)(self.context)
    }

    /// Sets the paused flag.
    pub(crate) fn set_paused(&self, paused: bool) -> Result<(), PluginError> {
        (self.fns.set_paused)(self.context, paused)
    }

    /// Toggles pause and returns the new state.
    pub(crate) fn toggle_pause(&self) -> Result<bool, PluginError> {
        (self.fns.toggle_pause)(self.context)
    }

    /// Sets the current simulation speed.
    pub(crate) fn set_speed(&self, speed: u32) -> Result<(), PluginError> {
        (self.fns.set_speed)(self.context, speed)
    }

    /// Sets the currently active tool.
    pub(crate) fn set_active_tool(
        &self,
        tool_id: Option<&ContentId>,
    ) -> Result<(), PluginError> {
        (self.fns.set_active_tool)(
            self.context,
            tool_id.as_ref().map(|id| id.as_str()),
        )
    }

    /// Writes one log line through the current runtime host.
    pub(crate) fn write_log(&self, level: u32, message: &str) -> Result<(), PluginError> {
        (self.fns.write_log)(self.context, level, message)
    }
}

/// Builds one runtime-host binding backed by ABI callbacks.
pub(crate) unsafe fn abi_runtime_host_binding(
    host: *mut FluxRuntimeHost,
) -> Option<RuntimeHostBinding> {
    let host = host.as_ref()?;
    Some(RuntimeHostBinding::new(host as *const FluxRuntimeHost as *mut c_void, &ABI_HOST_FNS))
}

static ABI_HOST_FNS: RuntimeHostFns = RuntimeHostFns {
    entity_place: abi_entity_place,
    entity_remove: abi_entity_remove,
    entity_remove_at: abi_entity_remove_at,
    entity_set_rotation: abi_entity_set_rotation,
    entity_set_enabled: abi_entity_set_enabled,
    entity_set_label: abi_entity_set_label,
    gas_add: abi_gas_add,
    gas_remove: abi_gas_remove,
    gas_clear_cell: abi_gas_clear_cell,
    gas_pressure_at: abi_gas_pressure_at,
    gas_amount_at: abi_gas_amount_at,
    submit_hud_line: abi_submit_hud_line,
    submit_overlay_frame: abi_submit_overlay_frame,
    write_save_chunk: abi_write_save_chunk,
    read_save_chunk: abi_read_save_chunk,
    delete_save_chunk: abi_delete_save_chunk,
    time_snapshot: abi_time_snapshot,
    set_paused: abi_set_paused,
    toggle_pause: abi_toggle_pause,
    set_speed: abi_set_speed,
    set_active_tool: abi_set_active_tool,
    write_log: abi_write_log,
};

fn abi_entity_place(
    context: *mut c_void,
    kind_id: &str,
    origin_x: u32,
    origin_y: u32,
    rotation: u32,
) -> Result<u32, PluginError> {
    let host = abi_host(context, "entities.place")?;
    let callback = host
        .entity_place_fn
        .ok_or(PluginError::Unsupported("entities.place"))?;
    let mut entity_id = 0u32;
    unsafe {
        callback(
            host.context,
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

fn abi_entity_remove(context: *mut c_void, entity_id: u32) -> Result<(), PluginError> {
    let host = abi_host(context, "entities.remove")?;
    let callback = host
        .entity_remove_fn
        .ok_or(PluginError::Unsupported("entities.remove"))?;
    unsafe { callback(host.context, entity_id) }
        .into_result()
        .map_err(status_error)
}

fn abi_entity_remove_at(
    context: *mut c_void,
    x: u32,
    y: u32,
    layer_id: &str,
) -> Result<Option<u32>, PluginError> {
    let host = abi_host(context, "entities.remove_at")?;
    let callback = host
        .entity_remove_at_fn
        .ok_or(PluginError::Unsupported("entities.remove_at"))?;
    let mut removed = 0u32;
    unsafe {
        callback(
            host.context,
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

fn abi_entity_set_rotation(
    context: *mut c_void,
    entity_id: u32,
    rotation: u32,
) -> Result<(), PluginError> {
    let host = abi_host(context, "entities.set_rotation")?;
    let callback = host
        .entity_set_rotation_fn
        .ok_or(PluginError::Unsupported("entities.set_rotation"))?;
    unsafe { callback(host.context, entity_id, rotation) }
        .into_result()
        .map_err(status_error)
}

fn abi_entity_set_enabled(
    context: *mut c_void,
    entity_id: u32,
    enabled: bool,
) -> Result<(), PluginError> {
    let host = abi_host(context, "entities.set_enabled")?;
    let callback = host
        .entity_set_enabled_fn
        .ok_or(PluginError::Unsupported("entities.set_enabled"))?;
    unsafe { callback(host.context, entity_id, enabled as u8) }
        .into_result()
        .map_err(status_error)
}

fn abi_entity_set_label(
    context: *mut c_void,
    entity_id: u32,
    label: &str,
) -> Result<(), PluginError> {
    let host = abi_host(context, "entities.set_label")?;
    let callback = host
        .entity_set_label_fn
        .ok_or(PluginError::Unsupported("entities.set_label"))?;
    unsafe { callback(host.context, entity_id, FluxUtf8Slice::from_str(label)) }
        .into_result()
        .map_err(status_error)
}

fn abi_gas_add(
    context: *mut c_void,
    x: u32,
    y: u32,
    substance: &str,
    amount: u32,
    velocity: Vec2,
) -> Result<u32, PluginError> {
    let host = abi_host(context, "gases.add")?;
    let callback = host
        .gas_add_fn
        .ok_or(PluginError::Unsupported("gases.add"))?;
    let mut added = 0u32;
    unsafe {
        callback(
            host.context,
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

fn abi_gas_remove(
    context: *mut c_void,
    x: u32,
    y: u32,
    substance: &str,
    amount: u32,
) -> Result<u32, PluginError> {
    let host = abi_host(context, "gases.remove")?;
    let callback = host
        .gas_remove_fn
        .ok_or(PluginError::Unsupported("gases.remove"))?;
    let mut removed = 0u32;
    unsafe {
        callback(
            host.context,
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

fn abi_gas_clear_cell(context: *mut c_void, x: u32, y: u32) -> Result<(), PluginError> {
    let host = abi_host(context, "gases.clear_cell")?;
    let callback = host
        .gas_clear_cell_fn
        .ok_or(PluginError::Unsupported("gases.clear_cell"))?;
    unsafe { callback(host.context, x, y) }
        .into_result()
        .map_err(status_error)
}

fn abi_gas_pressure_at(context: *mut c_void, x: u32, y: u32) -> Result<f32, PluginError> {
    let host = abi_host(context, "gases.pressure_at")?;
    let callback = host
        .gas_pressure_at_fn
        .ok_or(PluginError::Unsupported("gases.pressure_at"))?;
    let mut pressure = 0.0f32;
    unsafe { callback(host.context, x, y, &mut pressure) }
        .into_result()
        .map_err(status_error)?;
    Ok(pressure)
}

fn abi_gas_amount_at(
    context: *mut c_void,
    x: u32,
    y: u32,
    substance: &str,
) -> Result<u32, PluginError> {
    let host = abi_host(context, "gases.amount_at")?;
    let callback = host
        .gas_amount_at_fn
        .ok_or(PluginError::Unsupported("gases.amount_at"))?;
    let mut amount = 0u32;
    unsafe {
        callback(
            host.context,
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

fn abi_submit_hud_line(
    context: *mut c_void,
    block_id: &str,
    title: &str,
    line: &str,
    sort_order: i32,
) -> Result<(), PluginError> {
    let host = abi_host(context, "ui.add_hud_line")?;
    let callback = host
        .submit_hud_block_fn
        .ok_or(PluginError::Unsupported("ui.add_hud_line"))?;
    unsafe {
        callback(
            host.context,
            FluxUtf8Slice::from_str(block_id),
            FluxUtf8Slice::from_str(title),
            FluxUtf8Slice::from_str(line),
            sort_order,
        )
    }
    .into_result()
    .map_err(status_error)
}

fn abi_submit_overlay_frame(
    context: *mut c_void,
    width: u32,
    height: u32,
    rgba8: &[u8],
) -> Result<(), PluginError> {
    let host = abi_host(context, "overlays.submit_frame")?;
    let callback = host
        .submit_overlay_frame_fn
        .ok_or(PluginError::Unsupported("overlays.submit_frame"))?;
    unsafe { callback(host.context, width, height, rgba8.as_ptr(), rgba8.len()) }
        .into_result()
        .map_err(status_error)
}

fn abi_write_save_chunk(
    context: *mut c_void,
    chunk_id: &str,
    version: u32,
    bytes: &[u8],
) -> Result<(), PluginError> {
    let host = abi_host(context, "save.write_bytes")?;
    let callback = host
        .write_save_chunk_fn
        .ok_or(PluginError::Unsupported("save.write_bytes"))?;
    unsafe {
        callback(
            host.context,
            FluxUtf8Slice::from_str(chunk_id),
            version,
            bytes.as_ptr(),
            bytes.len(),
        )
    }
    .into_result()
    .map_err(status_error)
}

fn abi_read_save_chunk(
    context: *mut c_void,
    chunk_id: &str,
) -> Result<Option<RuntimeSaveChunkData>, PluginError> {
    let host = abi_host(context, "save.read_chunk")?;
    let callback = host
        .read_save_chunk_fn
        .ok_or(PluginError::Unsupported("save.read_chunk"))?;
    let mut version = 0u32;
    let mut len = 0usize;
    let status = unsafe {
        callback(
            host.context,
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
        callback(
            host.context,
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
    Ok(Some(RuntimeSaveChunkData { version, bytes }))
}

fn abi_delete_save_chunk(context: *mut c_void, chunk_id: &str) -> Result<(), PluginError> {
    let host = abi_host(context, "save.delete_chunk")?;
    let callback = host
        .delete_save_chunk_fn
        .ok_or(PluginError::Unsupported("save.delete_chunk"))?;
    unsafe { callback(host.context, FluxUtf8Slice::from_str(chunk_id)) }
        .into_result()
        .map_err(status_error)
}

fn abi_time_snapshot(context: *mut c_void) -> Result<RuntimeTimeSnapshot, PluginError> {
    let host = abi_host(context, "time.snapshot")?;
    let callback = host
        .get_time_snapshot_fn
        .ok_or(PluginError::Unsupported("time.snapshot"))?;
    let mut snapshot = FluxTimeSnapshot::default();
    unsafe { callback(host.context, &mut snapshot) }
        .into_result()
        .map_err(status_error)?;
    Ok(RuntimeTimeSnapshot {
        tick: snapshot.tick,
        paused: snapshot.paused != 0,
        speed: snapshot.speed,
        delta_seconds: snapshot.delta_seconds,
    })
}

fn abi_set_paused(context: *mut c_void, paused: bool) -> Result<(), PluginError> {
    let host = abi_host(context, "time.set_paused")?;
    let callback = host
        .set_paused_fn
        .ok_or(PluginError::Unsupported("time.set_paused"))?;
    unsafe { callback(host.context, paused as u8) }
        .into_result()
        .map_err(status_error)
}

fn abi_toggle_pause(context: *mut c_void) -> Result<bool, PluginError> {
    let host = abi_host(context, "time.toggle_pause")?;
    let callback = host
        .toggle_pause_fn
        .ok_or(PluginError::Unsupported("time.toggle_pause"))?;
    let mut paused = 0u8;
    unsafe { callback(host.context, &mut paused) }
        .into_result()
        .map_err(status_error)?;
    Ok(paused != 0)
}

fn abi_set_speed(context: *mut c_void, speed: u32) -> Result<(), PluginError> {
    let host = abi_host(context, "time.set_speed")?;
    let callback = host
        .set_speed_fn
        .ok_or(PluginError::Unsupported("time.set_speed"))?;
    unsafe { callback(host.context, speed) }
        .into_result()
        .map_err(status_error)
}

fn abi_set_active_tool(
    context: *mut c_void,
    tool_id: Option<&str>,
) -> Result<(), PluginError> {
    let host = abi_host(context, "ui.set_active_tool")?;
    let callback = host
        .set_active_tool_fn
        .ok_or(PluginError::Unsupported("ui.set_active_tool"))?;
    unsafe {
        callback(
            host.context,
            FluxUtf8Slice::from_str(tool_id.unwrap_or("")),
        )
    }
    .into_result()
    .map_err(status_error)
}

fn abi_write_log(context: *mut c_void, level: u32, message: &str) -> Result<(), PluginError> {
    let host = abi_host(context, "logger")?;
    let callback = host
        .write_log_fn
        .ok_or(PluginError::Unsupported("logger"))?;
    unsafe { callback(host.context, level, FluxUtf8Slice::from_str(message)) }
        .into_result()
        .map_err(status_error)
}

fn abi_host<'a>(
    context: *mut c_void,
    api_name: &'static str,
) -> Result<&'a FluxRuntimeHost, PluginError> {
    unsafe { context.cast::<FluxRuntimeHost>().as_ref() }
        .ok_or(PluginError::ApiUnavailable(api_name))
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
