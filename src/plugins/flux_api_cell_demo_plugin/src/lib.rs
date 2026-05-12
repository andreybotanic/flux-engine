use bevy_math::UVec2;
use flux_plugin_sdk::{
    declare_plugin, ContentId, EntityApi, EntityPlacement, LoggerApi, MouseButton, MouseCellEvent,
    Plugin, PluginError, PluginEvent, PluginInit, Registrar, Rotation, WorldApi,
};

const METAL_ENTITY_ID: &str = "flux.default.cell.metal";

pub struct ApiCellDemoPlugin {
    pub world: WorldApi,
    pub entities: EntityApi,
    pub log: LoggerApi,
    dragging: bool,
    last_cell: UVec2,
    metal_id: ContentId,
}

impl Plugin for ApiCellDemoPlugin {
    fn new(init: PluginInit) -> Result<Self, PluginError> {
        Ok(Self {
            world: init.world_api(),
            entities: init.entity_api(),
            log: init.logger_api(),
            dragging: false,
            last_cell: UVec2::ZERO,
            metal_id: ContentId::parse(METAL_ENTITY_ID).map_err(PluginError::from)?,
        })
    }

    fn register(&mut self, registrar: &mut Registrar<Self>) -> Result<(), PluginError> {
        registrar.subscribe(PluginEvent::MouseDownCell, Self::on_mouse_down)?;
        registrar.subscribe(PluginEvent::MouseMoveCell, Self::on_mouse_move)?;
        registrar.subscribe(PluginEvent::MouseUpCell, Self::on_mouse_up)?;
        Ok(())
    }
}

impl ApiCellDemoPlugin {
    fn on_mouse_down(&mut self, event: &MouseCellEvent) -> Result<(), PluginError> {
        if event.button != Some(MouseButton::Right) || !self.world.is_editable(event.cell) {
            return Ok(());
        }
        self.dragging = true;
        self.last_cell = event.cell;
        self.paint_cell(event.cell)
    }

    fn on_mouse_move(&mut self, event: &MouseCellEvent) -> Result<(), PluginError> {
        if !self.dragging || !self.world.contains(event.cell) {
            return Ok(());
        }
        for cell in self.world.ray_cells(self.last_cell, event.cell) {
            self.paint_cell(cell)?;
        }
        self.last_cell = event.cell;
        Ok(())
    }

    fn on_mouse_up(&mut self, event: &MouseCellEvent) -> Result<(), PluginError> {
        if !self.dragging {
            return Ok(());
        }
        self.dragging = false;
        if !self.world.contains(event.cell) {
            return Ok(());
        }
        for cell in self.world.ray_cells(self.last_cell, event.cell) {
            self.paint_cell(cell)?;
        }
        Ok(())
    }

    fn paint_cell(&mut self, cell: UVec2) -> Result<(), PluginError> {
        if !self.world.is_editable(cell) {
            return Ok(());
        }
        let _ = self.entities.place(
            self.metal_id.clone(),
            EntityPlacement {
                origin: cell,
                rotation: Rotation::Deg0,
            },
        )?;
        Ok(())
    }
}

declare_plugin!(ApiCellDemoPlugin);
