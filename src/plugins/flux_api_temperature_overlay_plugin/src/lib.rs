use flux_plugin_sdk::{
    declare_plugin, ContentId, LoggerApi, OverlayApi, OverlayDescriptor, OverlayGraph,
    OverlayImageInstance, OverlayImageSource, OverlayNode, OverlayNodeId, OverlayNodeKind,
    OverlayPlacement, OverlayRenderPolicy, Plugin, PluginError, PluginEvent, PluginInit,
    Registrar, RenderEntitiesNode, RenderImageNode, RenderOverlayEvent, Selector,
};
use bevy_math::{UVec2, Vec2, Vec4};

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
            graph: None,
        })?;
        registrar.subscribe(PluginEvent::RenderOverlay, Self::on_render_overlay)?;
        Ok(())
    }
}

impl TemperatureOverlayPlugin {
    fn on_render_overlay(&mut self, event: &RenderOverlayEvent) -> Result<(), PluginError> {
        if !is_temperature_overlay_request(event, &self.overlay_id) {
            return Ok(());
        }
        self.frame_counter = self.frame_counter.wrapping_add(1);
        self.overlays
            .submit_graph(build_temperature_graph(self.frame_counter)?)
    }
}

fn is_temperature_overlay_request(event: &RenderOverlayEvent, overlay_id: &ContentId) -> bool {
    event.overlay_id.as_str() == overlay_id.as_str()
}

fn build_temperature_graph(frame_counter: u32) -> Result<OverlayGraph, PluginError> {
    let map_node = OverlayNodeId::parse("temperature_map").map_err(PluginError::message)?;
    let solids_node = OverlayNodeId::parse("solids").map_err(PluginError::message)?;
    let compose_node = OverlayNodeId::parse("compose").map_err(PluginError::message)?;
    Ok(OverlayGraph {
        nodes: vec![
            OverlayNode {
                id: map_node.clone(),
                depends_on: Vec::new(),
                kind: OverlayNodeKind::RenderImage(RenderImageNode {
                    instances: vec![OverlayImageInstance {
                        image: OverlayImageSource::Rgba8 {
                            size_px: UVec2::new(OVERLAY_WIDTH, OVERLAY_HEIGHT),
                            rgba8: build_temperature_map(frame_counter),
                        },
                        placement: OverlayPlacement::GridLocal {
                            position_in_grid: Vec2::ZERO,
                            size_in_grid: Vec2::new(
                                OVERLAY_WIDTH as f32,
                                OVERLAY_HEIGHT as f32,
                            ),
                            rotation: 0.0,
                            origin: Vec2::ZERO,
                        },
                        tint: Vec4::ONE,
                    }],
                }),
            },
            OverlayNode {
                id: solids_node.clone(),
                depends_on: Vec::new(),
                kind: OverlayNodeKind::RenderEntities(RenderEntitiesNode {
                    selector: Selector::tag(
                        flux_plugin_sdk::ContentTag::parse("flux.default.tag.solid")
                            .map_err(PluginError::message)?,
                    ),
                    style: flux_plugin_sdk::OverlayEntityStyle {
                        tint: Some(Vec4::new(1.0, 1.0, 1.0, 0.82)),
                        sprite_override: Some(
                            flux_plugin_sdk::OverlayEntitySpriteOverride::Silhouette,
                        ),
                    },
                }),
            },
            OverlayNode {
                id: compose_node.clone(),
                depends_on: vec![map_node, solids_node],
                kind: OverlayNodeKind::Blend(flux_plugin_sdk::BlendNode {
                    mode: flux_plugin_sdk::OverlayBlendMode::AlphaOver,
                }),
            },
        ],
        output: compose_node,
    })
}

fn build_temperature_map(frame_counter: u32) -> Vec<u8> {
    let mut rgba8 = vec![0u8; (OVERLAY_WIDTH as usize) * (OVERLAY_HEIGHT as usize) * 4];
    for y in 0..OVERLAY_HEIGHT {
        for x in 0..OVERLAY_WIDTH {
            let wave = (((x as f32 * 0.12) + (frame_counter as f32 * 0.03)).sin() + 1.0) * 0.5;
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
    rgba8
}

declare_plugin!(TemperatureOverlayPlugin);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temperature_plugin_ignores_foreign_overlay_requests() {
        let target = ContentId::parse(OVERLAY_ID).expect("target overlay id");
        let foreign = ContentId::parse("flux.default.overlay.pipes").expect("foreign overlay id");
        let event = RenderOverlayEvent { overlay_id: foreign };
        assert!(!is_temperature_overlay_request(&event, &target));
    }
}
