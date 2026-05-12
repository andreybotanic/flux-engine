use flux_plugin_sdk::{
    declare_plugin, ContentId, LoggerApi, OverlayApi, OverlayDescriptor, OverlayFrame,
    OverlayRenderPolicy, Plugin, PluginError, PluginEvent, PluginInit, Registrar,
    RenderOverlayEvent,
};

const OVERLAY_ID: &str = "flux.api_temperature_overlay.overlay.temperature";
const OVERLAY_WIDTH: u32 = 102;
const OVERLAY_HEIGHT: u32 = 102;

pub struct TemperatureOverlayPlugin {
    pub overlays: OverlayApi,
    pub log: LoggerApi,
    frame_counter: u32,
    overlay_id: ContentId,
}

impl Plugin for TemperatureOverlayPlugin {
    fn new(init: PluginInit) -> Result<Self, PluginError> {
        Ok(Self {
            overlays: init.overlay_api(),
            log: init.logger_api(),
            frame_counter: 0,
            overlay_id: ContentId::parse(OVERLAY_ID).map_err(PluginError::from)?,
        })
    }

    fn register(&mut self, registrar: &mut Registrar<Self>) -> Result<(), PluginError> {
        registrar.register_overlay(OverlayDescriptor {
            id: self.overlay_id.clone(),
            label: "Temperature".to_string(),
            hotkey: None,
            render_policy: OverlayRenderPolicy::PluginControlled,
        })?;
        registrar.subscribe(PluginEvent::RenderOverlay, Self::on_render_overlay)?;
        Ok(())
    }
}

impl TemperatureOverlayPlugin {
    fn on_render_overlay(&mut self, _event: &RenderOverlayEvent) -> Result<(), PluginError> {
        self.frame_counter = self.frame_counter.wrapping_add(1);
        let mut rgba8 = vec![0u8; (OVERLAY_WIDTH as usize) * (OVERLAY_HEIGHT as usize) * 4];
        for y in 0..OVERLAY_HEIGHT {
            for x in 0..OVERLAY_WIDTH {
                let wave =
                    (((x as f32 * 0.12) + (self.frame_counter as f32 * 0.03)).sin() + 1.0) * 0.5;
                let vertical = y as f32 / (OVERLAY_HEIGHT - 1) as f32;
                let heat = (wave * 0.55 + vertical * 0.45).clamp(0.0, 1.0);
                let cold = 1.0 - heat;
                let index = ((y * OVERLAY_WIDTH + x) as usize) * 4;
                rgba8[index] = (heat * 255.0).round().clamp(0.0, 255.0) as u8;
                rgba8[index + 1] = ((0.18 + cold * 0.25) * 255.0).round().clamp(0.0, 255.0) as u8;
                rgba8[index + 2] = (cold * 255.0).round().clamp(0.0, 255.0) as u8;
                rgba8[index + 3] = (0.48_f32 * 255.0).round() as u8;
            }
        }
        self.overlays.submit_frame(OverlayFrame {
            overlay_id: self.overlay_id.clone(),
            width: OVERLAY_WIDTH,
            height: OVERLAY_HEIGHT,
            rgba8,
        })
    }
}

declare_plugin!(TemperatureOverlayPlugin);
