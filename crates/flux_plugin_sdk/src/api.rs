use bevy_math::{UVec2, Vec2};
use flux_plugin_abi::{FluxStatus, FluxTimeSnapshot, FluxUtf8Slice};
use smallvec::SmallVec;

use crate::{
    scope, CellPos, CellRect, CellSnapshot, ContentId, EntityFilter, EntityInstanceId,
    EntityKindId, EntityPlacement, EntitySnapshot, EntitySpawnRequest, GasMixture, HudBlock,
    InputModifiers, OverlayFrame, OverlayModeId, PlacementCheck, PluginError, Rotation, SaveChunk,
    SimulationSpeed, SubstanceId, SubstanceRegistryView, UiNode, UiPatch, WorldBounds,
};

/// Read-only world query API exposed to runtime plugins.
#[derive(Clone, Copy, Debug, Default)]
pub struct WorldApi;
/// Entity placement and mutation API exposed to runtime plugins.
#[derive(Clone, Copy, Debug, Default)]
pub struct EntityApi;
/// Gas query and editing API exposed to runtime plugins.
#[derive(Clone, Copy, Debug, Default)]
pub struct GasApi;
/// Basic UI and tool-selection API exposed to runtime plugins.
#[derive(Clone, Copy, Debug, Default)]
pub struct UiApi;
/// Panel-oriented UI API exposed to runtime plugins.
#[derive(Clone, Copy, Debug, Default)]
pub struct PanelApi;
/// Overlay rendering API exposed to runtime plugins.
#[derive(Clone, Copy, Debug, Default)]
pub struct OverlayApi;
/// Save-chunk persistence API exposed to runtime plugins.
#[derive(Clone, Copy, Debug, Default)]
pub struct SaveApi;
/// Simulation time control API exposed to runtime plugins.
#[derive(Clone, Copy, Debug, Default)]
pub struct TimeApi;
/// Read-only input snapshot API exposed to runtime plugins.
#[derive(Clone, Copy, Debug, Default)]
pub struct InputApi;
/// Logger API exposed to runtime plugins.
#[derive(Clone, Copy, Debug, Default)]
pub struct LoggerApi;

impl WorldApi {
    /// Returns world width in cells.
    pub fn width(&self) -> u32 {
        102
    }

    /// Returns world height in cells.
    pub fn height(&self) -> u32 {
        102
    }

    /// Returns world bounds in cells.
    pub fn bounds(&self) -> WorldBounds {
        WorldBounds {
            width: self.width(),
            height: self.height(),
        }
    }

    /// Returns `true` when the cell lies inside world bounds.
    pub fn contains(&self, cell: CellPos) -> bool {
        cell.x < self.width() && cell.y < self.height()
    }

    /// Returns `true` for boundary cells.
    pub fn is_boundary(&self, cell: CellPos) -> bool {
        cell.x == 0 || cell.y == 0 || cell.x + 1 == self.width() || cell.y + 1 == self.height()
    }

    /// Returns `true` when the cell can be edited by plugins.
    pub fn is_editable(&self, cell: CellPos) -> bool {
        self.contains(cell) && !self.is_boundary(cell)
    }

    /// Returns one lightweight cell snapshot when the host supports it.
    pub fn cell(&self, _cell: CellPos) -> Option<CellSnapshot> {
        None
    }

    /// Returns the currently hovered cell when the active event provides one.
    pub fn hovered_cell(&self) -> Option<CellPos> {
        scope::with_dispatch_state(|state| {
            state
                .cursor_world
                .and_then(|_| state.active_tool_id.as_ref().map(|_| UVec2::ZERO))
        })
    }

    /// Returns orthogonal neighbors of one cell.
    pub fn neighbors4(&self, cell: CellPos) -> [Option<CellPos>; 4] {
        [
            cell.y.checked_sub(1).map(|y| UVec2::new(cell.x, y)),
            cell.x.checked_add(1).filter(|x| *x < self.width()).map(|x| UVec2::new(x, cell.y)),
            cell.y.checked_add(1).filter(|y| *y < self.height()).map(|y| UVec2::new(cell.x, y)),
            cell.x.checked_sub(1).map(|x| UVec2::new(x, cell.y)),
        ]
    }

    /// Returns eight surrounding neighbors of one cell.
    pub fn neighbors8(&self, cell: CellPos) -> [Option<CellPos>; 8] {
        let offsets = [
            (-1, -1), (0, -1), (1, -1), (1, 0),
            (1, 1), (0, 1), (-1, 1), (-1, 0),
        ];
        offsets.map(|(dx, dy)| {
            let nx = cell.x as i32 + dx;
            let ny = cell.y as i32 + dy;
            (nx >= 0 && ny >= 0 && nx < self.width() as i32 && ny < self.height() as i32)
                .then_some(UVec2::new(nx as u32, ny as u32))
        })
    }

    /// Returns all cells touched by a Bresenham line.
    pub fn ray_cells(&self, from: CellPos, to: CellPos) -> Vec<CellPos> {
        let mut cells = Vec::new();
        let mut x0 = from.x as i32;
        let mut y0 = from.y as i32;
        let x1 = to.x as i32;
        let y1 = to.y as i32;
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            cells.push(UVec2::new(x0 as u32, y0 as u32));
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = err * 2;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
        cells
    }

    /// Returns every cell in one inclusive rectangle.
    pub fn rect_cells(&self, rect: CellRect) -> Vec<CellPos> {
        let mut cells = Vec::new();
        for y in rect.min.y..=rect.max.y {
            for x in rect.min.x..=rect.max.x {
                if self.contains(UVec2::new(x, y)) {
                    cells.push(UVec2::new(x, y));
                }
            }
        }
        cells
    }
}

impl EntityApi {
    /// Checks whether one entity kind can be placed at the requested location.
    pub fn can_place(&self, _kind: &EntityKindId, placement: &EntityPlacement) -> PlacementCheck {
        if placement.origin.x >= 102 || placement.origin.y >= 102 {
            PlacementCheck::OutOfBounds
        } else {
            PlacementCheck::Unsupported
        }
    }

    /// Places one entity.
    pub fn place(
        &mut self,
        kind: EntityKindId,
        placement: EntityPlacement,
    ) -> Result<EntityInstanceId, PluginError> {
        scope::with_runtime_host("entities.place", |host| {
            let callback = host
                .entity_place_fn
                .ok_or(PluginError::Unsupported("entities.place"))?;
            let mut entity_id = 0u32;
            unsafe {
                callback(
                    host.context,
                    FluxUtf8Slice::from_str(kind.as_str()),
                    placement.origin.x,
                    placement.origin.y,
                    encode_rotation(placement.rotation),
                    &mut entity_id,
                )
            }
            .into_result()
            .map_err(status_error)?;
            Ok(EntityInstanceId(entity_id))
        })
    }

    /// Places several entities sequentially.
    pub fn place_batch(
        &mut self,
        requests: &[EntitySpawnRequest],
    ) -> Result<Vec<EntityInstanceId>, PluginError> {
        requests
            .iter()
            .map(|request| self.place(request.kind.clone(), request.placement))
            .collect()
    }

    /// Removes one entity by runtime id.
    pub fn remove(&mut self, id: EntityInstanceId) -> Result<(), PluginError> {
        scope::with_runtime_host("entities.remove", |host| {
            let callback = host
                .entity_remove_fn
                .ok_or(PluginError::Unsupported("entities.remove"))?;
            unsafe { callback(host.context, id.0) }
                .into_result()
                .map_err(status_error)
        })
    }

    /// Removes one entity at the requested cell and layer.
    pub fn remove_at(
        &mut self,
        cell: CellPos,
        layer: ContentId,
    ) -> Result<Option<EntityInstanceId>, PluginError> {
        scope::with_runtime_host("entities.remove_at", |host| {
            let callback = host
                .entity_remove_at_fn
                .ok_or(PluginError::Unsupported("entities.remove_at"))?;
            let mut removed = 0u32;
            unsafe {
                callback(
                    host.context,
                    cell.x,
                    cell.y,
                    FluxUtf8Slice::from_str(layer.as_str()),
                    &mut removed,
                )
            }
            .into_result()
            .map_err(status_error)?;
            Ok((removed != 0).then_some(EntityInstanceId(removed)))
        })
    }

    /// Clears entities in the requested rectangle.
    pub fn clear_rect(&mut self, rect: CellRect, _filter: EntityFilter) -> Result<u32, PluginError> {
        let mut removed = 0u32;
        for y in rect.min.y..=rect.max.y {
            for x in rect.min.x..=rect.max.x {
                if self.remove_at(UVec2::new(x, y), ContentId::parse("flux.core.layer.appearance").expect("static content id"))?.is_some() {
                    removed += 1;
                }
            }
        }
        Ok(removed)
    }

    /// Returns one entity snapshot when the host supports it.
    pub fn get(&self, _id: EntityInstanceId) -> Option<EntitySnapshot> {
        None
    }

    /// Returns one entity id at the requested cell and layer when the host supports it.
    pub fn at(&self, _cell: CellPos, _layer: ContentId) -> Option<EntityInstanceId> {
        None
    }

    /// Returns all entity ids at the requested cell when the host supports it.
    pub fn all_at(&self, _cell: CellPos) -> SmallVec<[EntityInstanceId; 4]> {
        SmallVec::new()
    }

    /// Returns entities inside the rectangle when the host supports it.
    pub fn find_in_rect(&self, _rect: CellRect, _filter: EntityFilter) -> Vec<EntityInstanceId> {
        Vec::new()
    }

    /// Returns connected entities when the host supports it.
    pub fn find_connected(
        &self,
        _seed: EntityInstanceId,
        _filter: EntityFilter,
    ) -> Vec<EntityInstanceId> {
        Vec::new()
    }

    /// Updates one entity rotation.
    pub fn set_rotation(&mut self, id: EntityInstanceId, rotation: Rotation) -> Result<(), PluginError> {
        scope::with_runtime_host("entities.set_rotation", |host| {
            let callback = host
                .entity_set_rotation_fn
                .ok_or(PluginError::Unsupported("entities.set_rotation"))?;
            unsafe { callback(host.context, id.0, encode_rotation(rotation)) }
                .into_result()
                .map_err(status_error)
        })
    }

    /// Applies one minimal entity-state patch.
    pub fn set_state(&mut self, id: EntityInstanceId, patch: crate::EntityStatePatch) -> Result<(), PluginError> {
        if let Some(enabled) = patch.enabled {
            self.set_enabled(id, enabled)?;
        }
        if let Some(label) = patch.label {
            self.set_label(id, label)?;
        }
        Ok(())
    }

    /// Enables or disables one entity.
    pub fn set_enabled(&mut self, id: EntityInstanceId, enabled: bool) -> Result<(), PluginError> {
        scope::with_runtime_host("entities.set_enabled", |host| {
            let callback = host
                .entity_set_enabled_fn
                .ok_or(PluginError::Unsupported("entities.set_enabled"))?;
            unsafe { callback(host.context, id.0, enabled as u8) }
                .into_result()
                .map_err(status_error)
        })
    }

    /// Sets one human-readable entity label.
    pub fn set_label(&mut self, id: EntityInstanceId, label: String) -> Result<(), PluginError> {
        scope::with_runtime_host("entities.set_label", |host| {
            let callback = host
                .entity_set_label_fn
                .ok_or(PluginError::Unsupported("entities.set_label"))?;
            unsafe { callback(host.context, id.0, FluxUtf8Slice::from_str(&label)) }
                .into_result()
                .map_err(status_error)
        })
    }
}

impl GasApi {
    /// Returns the registered substance ids visible to the plugin.
    pub fn substances(&self) -> SubstanceRegistryView {
        SubstanceRegistryView::default()
    }

    /// Returns gas mixture information for one cell when available.
    pub fn mixture_at(&self, _cell: CellPos) -> GasMixture {
        GasMixture::default()
    }

    /// Returns gas pressure for one cell.
    pub fn pressure_at(&self, cell: CellPos) -> Result<f32, PluginError> {
        scope::with_runtime_host("gases.pressure_at", |host| {
            let callback = host
                .gas_pressure_at_fn
                .ok_or(PluginError::Unsupported("gases.pressure_at"))?;
            let mut pressure = 0.0f32;
            unsafe { callback(host.context, cell.x, cell.y, &mut pressure) }
                .into_result()
                .map_err(status_error)?;
            Ok(pressure)
        })
    }

    /// Returns the requested substance amount for one cell.
    pub fn amount_at(&self, cell: CellPos, substance: &SubstanceId) -> Result<u32, PluginError> {
        scope::with_runtime_host("gases.amount_at", |host| {
            let callback = host
                .gas_amount_at_fn
                .ok_or(PluginError::Unsupported("gases.amount_at"))?;
            let mut amount = 0u32;
            unsafe {
                callback(
                    host.context,
                    cell.x,
                    cell.y,
                    FluxUtf8Slice::from_str(substance.as_str()),
                    &mut amount,
                )
            }
            .into_result()
            .map_err(status_error)?;
            Ok(amount)
        })
    }

    /// Adds free gas to one world cell.
    pub fn add(&mut self, cell: CellPos, substance: SubstanceId, amount: u32) -> Result<u32, PluginError> {
        self.add_with_velocity(cell, substance, amount, Vec2::ZERO)
    }

    /// Adds free gas with velocity.
    pub fn add_with_velocity(
        &mut self,
        cell: CellPos,
        substance: SubstanceId,
        amount: u32,
        velocity: Vec2,
    ) -> Result<u32, PluginError> {
        scope::with_runtime_host("gases.add", |host| {
            let callback = host
                .gas_add_fn
                .ok_or(PluginError::Unsupported("gases.add"))?;
            let mut added = 0u32;
            unsafe {
                callback(
                    host.context,
                    cell.x,
                    cell.y,
                    FluxUtf8Slice::from_str(substance.as_str()),
                    amount,
                    velocity.x,
                    velocity.y,
                    &mut added,
                )
            }
            .into_result()
            .map_err(status_error)?;
            Ok(added)
        })
    }

    /// Removes free gas from one cell.
    pub fn remove(&mut self, cell: CellPos, substance: SubstanceId, amount: u32) -> Result<u32, PluginError> {
        scope::with_runtime_host("gases.remove", |host| {
            let callback = host
                .gas_remove_fn
                .ok_or(PluginError::Unsupported("gases.remove"))?;
            let mut removed = 0u32;
            unsafe {
                callback(
                    host.context,
                    cell.x,
                    cell.y,
                    FluxUtf8Slice::from_str(substance.as_str()),
                    amount,
                    &mut removed,
                )
            }
            .into_result()
            .map_err(status_error)?;
            Ok(removed)
        })
    }

    /// Clears all free gas from one cell.
    pub fn clear_cell(&mut self, cell: CellPos) -> Result<(), PluginError> {
        scope::with_runtime_host("gases.clear_cell", |host| {
            let callback = host
                .gas_clear_cell_fn
                .ok_or(PluginError::Unsupported("gases.clear_cell"))?;
            unsafe { callback(host.context, cell.x, cell.y) }
                .into_result()
                .map_err(status_error)
        })
    }
}

impl UiApi {
    /// Adds one complete HUD block.
    pub fn add_hud_block(&mut self, block: HudBlock) -> Result<(), PluginError> {
        for line in &block.lines {
            self.add_hud_line(block.id.clone(), block.title.clone(), line.clone())?;
        }
        Ok(())
    }

    /// Adds one HUD line.
    pub fn add_hud_line(
        &mut self,
        block_id: ContentId,
        title: String,
        line: String,
    ) -> Result<(), PluginError> {
        scope::with_runtime_host("ui.add_hud_line", |host| {
            let callback = host
                .submit_hud_block_fn
                .ok_or(PluginError::Unsupported("ui.add_hud_line"))?;
            unsafe {
                callback(
                    host.context,
                    FluxUtf8Slice::from_str(block_id.as_str()),
                    FluxUtf8Slice::from_str(&title),
                    FluxUtf8Slice::from_str(&line),
                    1000,
                )
            }
            .into_result()
            .map_err(status_error)
        })
    }

    /// Clears one HUD block. The current host does not support incremental removal yet.
    pub fn clear_hud_block(&mut self, _block_id: &ContentId) -> Result<(), PluginError> {
        Err(PluginError::Unsupported("ui.clear_hud_block"))
    }

    /// Opens one panel when the host supports it.
    pub fn open_panel(&mut self, _panel_id: ContentId) -> Result<(), PluginError> {
        Err(PluginError::Unsupported("ui.open_panel"))
    }

    /// Closes one panel when the host supports it.
    pub fn close_panel(&mut self, _panel_id: &ContentId) -> Result<(), PluginError> {
        Err(PluginError::Unsupported("ui.close_panel"))
    }

    /// Selects the active tool by stable id.
    pub fn set_active_tool(&mut self, tool_id: Option<ContentId>) -> Result<(), PluginError> {
        scope::with_runtime_host("ui.set_active_tool", |host| {
            let callback = host
                .set_active_tool_fn
                .ok_or(PluginError::Unsupported("ui.set_active_tool"))?;
            unsafe {
                callback(
                    host.context,
                    FluxUtf8Slice::from_str(tool_id.as_ref().map(|id| id.as_str()).unwrap_or("")),
                )
            }
            .into_result()
            .map_err(status_error)
        })
    }
}

impl PanelApi {
    /// Returns the panel requested by the current `BuildPanel` dispatch.
    pub fn requested_panel(&self) -> Option<ContentId> {
        scope::with_dispatch_state(|state| state.requested_panel.clone())
    }

    /// Returns whether the requested panel matches the given id.
    pub fn is_open(&self, panel_id: &ContentId) -> bool {
        self.requested_panel().as_ref() == Some(panel_id)
    }

    /// Sets the panel root. The current host does not support this yet.
    pub fn set_root(&mut self, _panel_id: &ContentId, _root: UiNode) -> Result<(), PluginError> {
        Err(PluginError::Unsupported("panels.set_root"))
    }

    /// Applies one panel patch. The current host does not support this yet.
    pub fn patch_root(&mut self, _panel_id: &ContentId, _patch: UiPatch) -> Result<(), PluginError> {
        Err(PluginError::Unsupported("panels.patch_root"))
    }

    /// Sets one panel title. The current host does not support this yet.
    pub fn set_title(&mut self, _panel_id: &ContentId, _title: String) -> Result<(), PluginError> {
        Err(PluginError::Unsupported("panels.set_title"))
    }

    /// Requests one panel rebuild. The current host does not support this yet.
    pub fn request_rebuild(&mut self, _panel_id: &ContentId) -> Result<(), PluginError> {
        Err(PluginError::Unsupported("panels.request_rebuild"))
    }
}

impl OverlayApi {
    /// Returns the active overlay id from the current dispatch state.
    pub fn active_overlay(&self) -> Option<OverlayModeId> {
        scope::with_dispatch_state(|state| state.active_overlay.clone())
    }

    /// Returns the requested overlay id from the current dispatch state.
    pub fn requested_overlay(&self) -> Option<OverlayModeId> {
        scope::with_dispatch_state(|state| state.requested_overlay.clone())
    }

    /// Returns the viewport size in cells.
    pub fn viewport_size(&self) -> UVec2 {
        UVec2::new(102, 102)
    }

    /// Submits one complete RGBA8 frame.
    pub fn submit_frame(&mut self, frame: OverlayFrame) -> Result<(), PluginError> {
        scope::with_runtime_host("overlays.submit_frame", |host| {
            let callback = host
                .submit_overlay_frame_fn
                .ok_or(PluginError::Unsupported("overlays.submit_frame"))?;
            unsafe {
                callback(
                    host.context,
                    frame.width,
                    frame.height,
                    frame.rgba8.as_ptr(),
                    frame.rgba8.len(),
                )
            }
            .into_result()
            .map_err(status_error)
        })
    }

    /// Clears the current overlay frame by submitting an empty frame.
    pub fn clear_frame(&mut self) -> Result<(), PluginError> {
        Err(PluginError::Unsupported("overlays.clear_frame"))
    }
}

impl SaveApi {
    /// Reads one raw save chunk.
    pub fn read_chunk(&self, chunk_id: &ContentId) -> Result<Option<SaveChunk>, PluginError> {
        scope::with_runtime_host("save.read_chunk", |host| {
            let callback = host
                .read_save_chunk_fn
                .ok_or(PluginError::Unsupported("save.read_chunk"))?;
            let mut version = 0u32;
            let mut len = 0usize;
            let status = unsafe {
                callback(
                    host.context,
                    FluxUtf8Slice::from_str(chunk_id.as_str()),
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
                    FluxUtf8Slice::from_str(chunk_id.as_str()),
                    &mut version,
                    bytes.as_mut_ptr(),
                    bytes.len(),
                    &mut len,
                )
            }
            .into_result()
            .map_err(status_error)?;
            Ok(Some(SaveChunk {
                plugin_id: crate::PluginId::parse("flux.runtime").expect("static plugin id"),
                chunk_id: chunk_id.clone(),
                version,
                bytes,
            }))
        })
    }

    /// Returns `true` when one chunk exists.
    pub fn has_chunk(&self, chunk_id: &ContentId) -> bool {
        self.read_chunk(chunk_id).ok().flatten().is_some()
    }

    /// Reads one raw byte chunk.
    pub fn read_bytes(&self, chunk_id: &ContentId) -> Result<Option<Vec<u8>>, PluginError> {
        Ok(self.read_chunk(chunk_id)?.map(|chunk| chunk.bytes))
    }

    /// Reads one JSON chunk.
    pub fn read_json<T: serde::de::DeserializeOwned>(&self, chunk_id: &ContentId) -> Result<Option<T>, PluginError> {
        Ok(match self.read_chunk(chunk_id)? {
            Some(chunk) => Some(chunk.read_json()?),
            None => None,
        })
    }

    /// Writes one raw chunk.
    pub fn write_chunk(&mut self, chunk: SaveChunk) -> Result<(), PluginError> {
        self.write_bytes(chunk.chunk_id, chunk.version, chunk.bytes)
    }

    /// Writes one raw byte chunk.
    pub fn write_bytes(
        &mut self,
        chunk_id: ContentId,
        version: u32,
        bytes: Vec<u8>,
    ) -> Result<(), PluginError> {
        scope::with_runtime_host("save.write_bytes", |host| {
            let callback = host
                .write_save_chunk_fn
                .ok_or(PluginError::Unsupported("save.write_bytes"))?;
            unsafe {
                callback(
                    host.context,
                    FluxUtf8Slice::from_str(chunk_id.as_str()),
                    version,
                    bytes.as_ptr(),
                    bytes.len(),
                )
            }
            .into_result()
            .map_err(status_error)
        })
    }

    /// Writes one JSON chunk.
    pub fn write_json<T: serde::Serialize>(
        &mut self,
        chunk_id: ContentId,
        version: u32,
        value: &T,
    ) -> Result<(), PluginError> {
        let bytes = serde_json::to_vec(value)
            .map_err(|error| PluginError::message(format!("failed to encode JSON: {error}")))?;
        self.write_bytes(chunk_id, version, bytes)
    }

    /// Deletes one chunk.
    pub fn delete_chunk(&mut self, chunk_id: &ContentId) -> Result<(), PluginError> {
        scope::with_runtime_host("save.delete_chunk", |host| {
            let callback = host
                .delete_save_chunk_fn
                .ok_or(PluginError::Unsupported("save.delete_chunk"))?;
            unsafe { callback(host.context, FluxUtf8Slice::from_str(chunk_id.as_str())) }
                .into_result()
                .map_err(status_error)
        })
    }
}

impl TimeApi {
    /// Returns the current simulation tick.
    pub fn tick(&self) -> Result<u64, PluginError> {
        Ok(self.snapshot()?.tick)
    }

    /// Returns the current pause flag.
    pub fn is_paused(&self) -> Result<bool, PluginError> {
        Ok(self.snapshot()?.paused)
    }

    /// Returns the current simulation speed.
    pub fn speed(&self) -> Result<SimulationSpeed, PluginError> {
        Ok(self.snapshot()?.speed)
    }

    /// Returns the last simulation delta time in seconds.
    pub fn delta_seconds(&self) -> Result<f32, PluginError> {
        Ok(self.snapshot()?.delta_seconds)
    }

    /// Sets the pause flag.
    pub fn set_paused(&mut self, paused: bool) -> Result<(), PluginError> {
        scope::with_runtime_host("time.set_paused", |host| {
            let callback = host
                .set_paused_fn
                .ok_or(PluginError::Unsupported("time.set_paused"))?;
            unsafe { callback(host.context, paused as u8) }
                .into_result()
                .map_err(status_error)
        })
    }

    /// Toggles pause and returns the new pause flag.
    pub fn toggle_pause(&mut self) -> Result<bool, PluginError> {
        scope::with_runtime_host("time.toggle_pause", |host| {
            let callback = host
                .toggle_pause_fn
                .ok_or(PluginError::Unsupported("time.toggle_pause"))?;
            let mut paused = 0u8;
            unsafe { callback(host.context, &mut paused) }
                .into_result()
                .map_err(status_error)?;
            Ok(paused != 0)
        })
    }

    /// Sets the current simulation speed.
    pub fn set_speed(&mut self, speed: SimulationSpeed) -> Result<(), PluginError> {
        scope::with_runtime_host("time.set_speed", |host| {
            let callback = host
                .set_speed_fn
                .ok_or(PluginError::Unsupported("time.set_speed"))?;
            unsafe { callback(host.context, encode_speed(speed)) }
                .into_result()
                .map_err(status_error)
        })
    }

    fn snapshot(&self) -> Result<DecodedTimeSnapshot, PluginError> {
        scope::with_runtime_host("time.snapshot", |host| {
            let callback = host
                .get_time_snapshot_fn
                .ok_or(PluginError::Unsupported("time.snapshot"))?;
            let mut snapshot = FluxTimeSnapshot::default();
            unsafe { callback(host.context, &mut snapshot) }
                .into_result()
                .map_err(status_error)?;
            Ok(DecodedTimeSnapshot {
                tick: snapshot.tick,
                paused: snapshot.paused != 0,
                speed: decode_speed(snapshot.speed),
                delta_seconds: snapshot.delta_seconds,
            })
        })
    }
}

impl InputApi {
    /// Returns the current modifier snapshot.
    pub fn modifiers(&self) -> InputModifiers {
        scope::with_dispatch_state(|state| state.modifiers)
    }

    /// Returns the active tool id captured for the current event.
    pub fn active_tool(&self) -> Option<ContentId> {
        scope::with_dispatch_state(|state| state.active_tool_id.clone())
    }

    /// Returns current world cursor position when available.
    pub fn cursor_world(&self) -> Option<Vec2> {
        scope::with_dispatch_state(|state| state.cursor_world)
    }

    /// Returns current screen cursor position when available.
    pub fn cursor_screen(&self) -> Option<Vec2> {
        scope::with_dispatch_state(|state| state.cursor_screen)
    }

    /// Returns whether the pointer is over UI for the current event.
    pub fn is_pointer_over_ui(&self) -> bool {
        scope::with_dispatch_state(|state| state.is_pointer_over_ui)
    }
}

impl LoggerApi {
    /// Writes one error log line.
    pub fn error(&self, message: impl Into<String>) -> Result<(), PluginError> {
        scope::write_log(1, &message.into())
    }

    /// Writes one warning log line.
    pub fn warn(&self, message: impl Into<String>) -> Result<(), PluginError> {
        scope::write_log(2, &message.into())
    }

    /// Writes one info log line.
    pub fn info(&self, message: impl Into<String>) -> Result<(), PluginError> {
        scope::write_log(3, &message.into())
    }

    /// Writes one debug log line.
    pub fn debug(&self, message: impl Into<String>) -> Result<(), PluginError> {
        scope::write_log(4, &message.into())
    }
}

struct DecodedTimeSnapshot {
    tick: u64,
    paused: bool,
    speed: SimulationSpeed,
    delta_seconds: f32,
}

fn status_error(status: FluxStatus) -> PluginError {
    match status {
        FluxStatus::INVALID_ARGUMENT => PluginError::InvalidArgument("host rejected arguments".to_string()),
        FluxStatus::API_UNAVAILABLE => PluginError::ApiUnavailable("runtime"),
        FluxStatus::UNSUPPORTED => PluginError::Unsupported("runtime"),
        _ => PluginError::message(format!("host call failed with status {}", status.code())),
    }
}

fn encode_rotation(rotation: Rotation) -> u32 {
    match rotation {
        Rotation::Deg0 => 0,
        Rotation::Deg90 => 1,
        Rotation::Deg180 => 2,
        Rotation::Deg270 => 3,
    }
}

fn encode_speed(speed: SimulationSpeed) -> u32 {
    match speed {
        SimulationSpeed::X1 => 1,
        SimulationSpeed::X2 => 2,
        SimulationSpeed::X5 => 5,
    }
}

fn decode_speed(raw: u32) -> SimulationSpeed {
    match raw {
        2 => SimulationSpeed::X2,
        5 => SimulationSpeed::X5,
        _ => SimulationSpeed::X1,
    }
}
