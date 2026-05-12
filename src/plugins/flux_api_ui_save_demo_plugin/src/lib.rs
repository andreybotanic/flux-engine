use flux_plugin_sdk::{
    declare_plugin, BuildHudForCellEvent, BuildPanelEvent, ContentId, LoggerApi, MouseButton,
    MouseCellEvent, PanelApi, PanelDescriptor, Plugin, PluginError, PluginEvent, PluginInit,
    Registrar, SaveApi, SaveChunkDescriptor, UiApi, UiNode, WorldBeforeSaveEvent,
    WorldLoadedEvent,
};

const WORLD_WIDTH: u32 = 102;
const WORLD_HEIGHT: u32 = 102;
const SAVE_CHUNK_ID: &str = "flux.api_ui_save_demo.save.counter";
const PANEL_ID: &str = "flux.api_ui_save_demo.panel.counter";
const HUD_BLOCK_ID: &str = "flux.api_ui_save_demo.hud.counter";

pub struct ApiUiSaveDemoPlugin {
    pub ui: UiApi,
    pub panels: PanelApi,
    pub save: SaveApi,
    pub log: LoggerApi,
    cell_counters: Vec<u32>,
    save_chunk_id: ContentId,
    panel_id: ContentId,
    hud_block_id: ContentId,
}

impl Plugin for ApiUiSaveDemoPlugin {
    fn new(init: PluginInit) -> Result<Self, PluginError> {
        Ok(Self {
            ui: init.ui_api(),
            panels: init.panel_api(),
            save: init.save_api(),
            log: init.logger_api(),
            cell_counters: vec![0; (WORLD_WIDTH * WORLD_HEIGHT) as usize],
            save_chunk_id: ContentId::parse(SAVE_CHUNK_ID).map_err(PluginError::from)?,
            panel_id: ContentId::parse(PANEL_ID).map_err(PluginError::from)?,
            hud_block_id: ContentId::parse(HUD_BLOCK_ID).map_err(PluginError::from)?,
        })
    }

    fn register(&mut self, registrar: &mut Registrar<Self>) -> Result<(), PluginError> {
        registrar.register_panel(PanelDescriptor {
            id: self.panel_id.clone(),
            title: "API Counter".to_string(),
            root: UiNode::Text {
                text: "Counter panel is rebuilt from plugin events.".to_string(),
            },
        })?;
        registrar.register_save_chunk(SaveChunkDescriptor {
            id: self.save_chunk_id.clone(),
            version: 1,
        })?;
        registrar.subscribe(PluginEvent::WorldLoaded, Self::on_world_loaded)?;
        registrar.subscribe(PluginEvent::WorldBeforeSave, Self::on_world_before_save)?;
        registrar.subscribe(PluginEvent::MouseDownCell, Self::on_mouse_down)?;
        registrar.subscribe(PluginEvent::BuildHudForCell, Self::on_build_hud_for_cell)?;
        registrar.subscribe(PluginEvent::BuildPanel, Self::on_build_panel)?;
        Ok(())
    }
}

impl ApiUiSaveDemoPlugin {
    fn on_world_loaded(&mut self, _event: &WorldLoadedEvent) -> Result<(), PluginError> {
        let Some(bytes) = self.save.read_bytes(&self.save_chunk_id)? else {
            return Ok(());
        };
        if bytes.len() != self.cell_counters.len() * 4 {
            return Ok(());
        }
        for (index, chunk) in bytes.chunks_exact(4).enumerate() {
            self.cell_counters[index] =
                u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        Ok(())
    }

    fn on_world_before_save(&mut self, _event: &WorldBeforeSaveEvent) -> Result<(), PluginError> {
        let mut bytes = Vec::with_capacity(self.cell_counters.len() * 4);
        for value in &self.cell_counters {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        self.save
            .write_bytes(self.save_chunk_id.clone(), 1, bytes)
    }

    fn on_mouse_down(&mut self, event: &MouseCellEvent) -> Result<(), PluginError> {
        if event.button != Some(MouseButton::Left) {
            return Ok(());
        }
        if let Some(index) = cell_index(event.cell.x, event.cell.y) {
            self.cell_counters[index] = self.cell_counters[index].saturating_add(1);
        }
        Ok(())
    }

    fn on_build_hud_for_cell(
        &mut self,
        event: &BuildHudForCellEvent,
    ) -> Result<(), PluginError> {
        let counter = cell_index(event.cell.x, event.cell.y)
            .and_then(|index| self.cell_counters.get(index).copied())
            .unwrap_or(0);
        self.ui.add_hud_line(
            self.hud_block_id.clone(),
            "API UI/Save Demo".to_string(),
            format!(
                "cell left-clicks {}, cell ({}, {})",
                counter, event.cell.x, event.cell.y
            ),
        )
    }

    fn on_build_panel(&mut self, _event: &BuildPanelEvent) -> Result<(), PluginError> {
        if let Some(requested) = self.panels.requested_panel() {
            let total_clicks: u32 = self.cell_counters.iter().copied().sum();
            let _ = self.log.info(format!(
                "panel '{}' requested, total left-clicks {}",
                requested, total_clicks
            ));
        }
        Ok(())
    }
}

fn cell_index(x: u32, y: u32) -> Option<usize> {
    (x < WORLD_WIDTH && y < WORLD_HEIGHT).then_some((y * WORLD_WIDTH + x) as usize)
}

declare_plugin!(ApiUiSaveDemoPlugin);
