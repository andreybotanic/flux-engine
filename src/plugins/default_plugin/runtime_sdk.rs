use std::{cell::RefCell, ptr};

use flux_plugin_sdk::{
    BuildHudForCellEvent, EntityApi, GasApi, HudBlock, InputApi, LoggerApi, OverlayApi,
    OverlayDescriptor, OverlayMaterialDescriptor, OverlayRenderPolicy, Plugin, PluginError,
    PluginEvent, PluginInit, RenderOverlayEvent, SaveApi, SimulationPausedChangedEvent,
    SimulationPreCellGasStepEvent, TimeApi, UiApi, WorldAfterSaveEvent, WorldApi,
    WorldBeforeSaveEvent,
};

use crate::{
    plugins::{
        default_plugin::pipe_runtime::{
            apply_gas_structures_pre_step, apply_pipe_network_step,
            build_pipe_debug_hud_lines_for_cell,
        },
        default_plugin::{
            build_pipes_overlay_graph, OVERLAY_MATERIAL_PIPE_HIGHLIGHT_ID, OVERLAY_PIPES_ID,
        },
        RuntimeHostContext,
    },
    ui::cell_inspector_model::{build_cell_inspector_blocks, CellInspectorBlockView},
};

thread_local! {
    static CURRENT_DEFAULT_PLUGIN_CONTEXT: RefCell<*mut ()> = const { RefCell::new(ptr::null_mut()) };
}

/// Runs one closure with the current built-in default-plugin runtime context bound.
pub(crate) fn with_runtime_context<T>(
    context: *mut RuntimeHostContext,
    f: impl FnOnce() -> T,
) -> T {
    CURRENT_DEFAULT_PLUGIN_CONTEXT.with(|current| {
        let previous = *current.borrow();
        *current.borrow_mut() = context.cast::<()>();
        let value = f();
        *current.borrow_mut() = previous;
        value
    })
}

fn with_bound_context<T>(
    api_name: &'static str,
    f: impl FnOnce(&mut RuntimeHostContext) -> Result<T, PluginError>,
) -> Result<T, PluginError> {
    CURRENT_DEFAULT_PLUGIN_CONTEXT.with(|current| {
        let ptr = *current.borrow();
        let Some(context) = (unsafe { ptr.cast::<RuntimeHostContext>().as_mut() }) else {
            return Err(PluginError::ApiUnavailable(api_name));
        };
        f(context)
    })
}

/// Built-in SDK runtime plugin that owns `flux.default` behavior.
pub struct FluxDefaultRuntimeSdkPlugin {
    pub world: WorldApi,
    pub entities: EntityApi,
    pub gases: GasApi,
    pub ui: UiApi,
    pub overlays: OverlayApi,
    pub save: SaveApi,
    pub time: TimeApi,
    pub input: InputApi,
    pub log: LoggerApi,
}

impl Plugin for FluxDefaultRuntimeSdkPlugin {
    fn new(init: PluginInit) -> Result<Self, PluginError> {
        Ok(Self {
            world: init.world_api(),
            entities: init.entity_api(),
            gases: init.gas_api(),
            ui: init.ui_api(),
            overlays: init.overlay_api(),
            save: init.save_api(),
            time: init.time_api(),
            input: init.input_api(),
            log: init.logger_api(),
        })
    }

    fn register(
        &mut self,
        registrar: &mut flux_plugin_sdk::Registrar<Self>,
    ) -> Result<(), PluginError> {
        registrar.register_overlay(OverlayDescriptor {
            id: flux_plugin_sdk::ContentId::parse(OVERLAY_PIPES_ID)
                .map_err(PluginError::message)?,
            label: "Pipes".to_string(),
            hotkey: Some("F3".to_string()),
            render_policy: OverlayRenderPolicy::PluginControlled,
            graph: None,
        })?;
        registrar.register_overlay_material(OverlayMaterialDescriptor {
            id: flux_plugin_sdk::ContentId::parse(OVERLAY_MATERIAL_PIPE_HIGHLIGHT_ID)
                .map_err(PluginError::message)?,
            label: "Pipe Highlight".to_string(),
            shader_path: "flux_default://shaders/pipe_highlight_material.wgsl".to_string(),
        })?;
        registrar.subscribe(PluginEvent::WorldCreated, Self::on_world_created)?;
        registrar.subscribe(PluginEvent::WorldLoaded, Self::on_world_loaded)?;
        registrar.subscribe(PluginEvent::WorldUnloaded, Self::on_world_unloaded)?;
        registrar.subscribe(PluginEvent::WorldBeforeSave, Self::on_world_before_save)?;
        registrar.subscribe(PluginEvent::WorldAfterSave, Self::on_world_after_save)?;
        registrar.subscribe(
            PluginEvent::SimulationPreCellGasStep,
            Self::on_simulation_pre_cell_gas_step,
        )?;
        registrar.subscribe(
            PluginEvent::SimulationPausedChanged,
            Self::on_simulation_paused_changed,
        )?;
        registrar.subscribe(PluginEvent::BuildHudForCell, Self::on_build_hud_for_cell)?;
        registrar.subscribe(PluginEvent::RenderOverlay, Self::on_render_overlay)?;
        Ok(())
    }
}

impl FluxDefaultRuntimeSdkPlugin {
    fn on_world_created(
        &mut self,
        _event: &flux_plugin_sdk::WorldCreatedEvent,
    ) -> Result<(), PluginError> {
        reset_runtime_state(false)
    }

    fn on_world_loaded(
        &mut self,
        _event: &flux_plugin_sdk::WorldLoadedEvent,
    ) -> Result<(), PluginError> {
        reset_runtime_state(false)
    }

    fn on_world_unloaded(
        &mut self,
        _event: &flux_plugin_sdk::WorldUnloadedEvent,
    ) -> Result<(), PluginError> {
        reset_runtime_state(true)
    }

    fn on_world_before_save(&mut self, _event: &WorldBeforeSaveEvent) -> Result<(), PluginError> {
        Ok(())
    }

    fn on_world_after_save(&mut self, _event: &WorldAfterSaveEvent) -> Result<(), PluginError> {
        Ok(())
    }

    fn on_simulation_pre_cell_gas_step(
        &mut self,
        _event: &SimulationPreCellGasStepEvent,
    ) -> Result<(), PluginError> {
        with_bound_context("flux.default.pre_gas_step", |context| {
            let structures = context
                .structures
                .as_deref()
                .ok_or(PluginError::ApiUnavailable("flux.default.structures"))?;
            let world = context
                .world
                .as_deref()
                .ok_or(PluginError::ApiUnavailable("flux.default.world"))?;
            let pipe_config = context
                .pipe_config
                .ok_or(PluginError::ApiUnavailable("flux.default.pipe_config"))?;
            let pipe_gas = context
                .pipe_gas
                .as_deref_mut()
                .ok_or(PluginError::ApiUnavailable("flux.default.pipe_gas"))?;
            let pipe_flux = context
                .pipe_flux
                .as_deref_mut()
                .ok_or(PluginError::ApiUnavailable("flux.default.pipe_flux"))?;
            let pipe_flow_visuals =
                context
                    .pipe_flow_visuals
                    .as_deref_mut()
                    .ok_or(PluginError::ApiUnavailable(
                        "flux.default.pipe_flow_visuals",
                    ))?;
            let gas = context
                .gas
                .as_deref_mut()
                .ok_or(PluginError::ApiUnavailable("flux.default.gas"))?;
            let perf = context
                .simulation_perf
                .as_deref_mut()
                .ok_or(PluginError::ApiUnavailable("flux.default.simulation_perf"))?;

            let pipe_started_at = std::time::Instant::now();
            let changed_by_pipes = apply_pipe_network_step(
                structures,
                pipe_gas,
                pipe_flux,
                gas,
                world,
                pipe_flow_visuals,
                pipe_config,
            );
            let pipe_elapsed_ms = pipe_started_at.elapsed().as_secs_f32() * 1000.0;
            perf.last_pipe_step_ms = pipe_elapsed_ms;
            perf.avg_pipe_step_ms = if perf.avg_pipe_step_ms <= f32::EPSILON {
                pipe_elapsed_ms
            } else {
                perf.avg_pipe_step_ms * 0.9 + pipe_elapsed_ms * 0.1
            };

            let changed_by_structures = apply_gas_structures_pre_step(structures, gas, world);
            if (changed_by_pipes || changed_by_structures)
                && context.gpu_state.as_deref().map(|_| true).unwrap_or(false)
            {
                if let Some(gpu_state) = context.gpu_state.as_deref_mut() {
                    gpu_state.mark_needs_full_upload();
                }
            }
            Ok(())
        })
    }

    fn on_simulation_paused_changed(
        &mut self,
        _event: &SimulationPausedChangedEvent,
    ) -> Result<(), PluginError> {
        Ok(())
    }

    fn on_build_hud_for_cell(&mut self, event: &BuildHudForCellEvent) -> Result<(), PluginError> {
        let blocks = with_bound_context("flux.default.build_hud", |context| {
            let world = context
                .world
                .as_deref()
                .ok_or(PluginError::ApiUnavailable("flux.default.world"))?;
            let gas = context
                .gas
                .as_deref()
                .ok_or(PluginError::ApiUnavailable("flux.default.gas"))?;
            let pipe_config = context
                .pipe_config
                .ok_or(PluginError::ApiUnavailable("flux.default.pipe_config"))?;
            let gas_registry = context.gas_registry;
            let world_cell_hud = context
                .world_cell_hud
                .ok_or(PluginError::ApiUnavailable("flux.default.world_cell_hud"))?;
            let cell_visual_layouts =
                context
                    .cell_visual_layouts
                    .ok_or(PluginError::ApiUnavailable(
                        "flux.default.cell_visual_layouts",
                    ))?;
            let structure_hud = context
                .structure_hud
                .ok_or(PluginError::ApiUnavailable("flux.default.structure_hud"))?;
            let structure_visuals =
                context
                    .structure_visuals
                    .ok_or(PluginError::ApiUnavailable(
                        "flux.default.structure_visuals",
                    ))?;
            let structures = context
                .structures
                .as_deref()
                .ok_or(PluginError::ApiUnavailable("flux.default.structures"))?;
            let pipe_gas = context
                .pipe_gas
                .as_deref()
                .ok_or(PluginError::ApiUnavailable("flux.default.pipe_gas"))?;
            let flow_state =
                context
                    .pipe_flow_visuals
                    .as_deref()
                    .ok_or(PluginError::ApiUnavailable(
                        "flux.default.pipe_flow_visuals",
                    ))?;
            let mut blocks = build_cell_inspector_blocks(
                event.cell,
                world,
                gas,
                pipe_config,
                gas_registry,
                world_cell_hud,
                cell_visual_layouts,
                structure_hud,
                structure_visuals,
                structures,
                pipe_gas,
                flow_state,
                false,
            );
            let debug_lines = build_pipe_debug_hud_lines_for_cell(
                event.cell,
                structures,
                pipe_gas,
                gas,
                pipe_config,
                flow_state,
            );
            if !debug_lines.is_empty() {
                blocks.push(CellInspectorBlockView {
                    title: "[DEBUG] Pipe/Vent".to_string(),
                    lines: debug_lines,
                });
            }
            Ok(blocks)
        })?;

        for (index, block) in blocks.into_iter().enumerate() {
            self.ui.add_hud_block(HudBlock {
                id: hud_block_id(index),
                title: block.title,
                lines: block.lines,
                sort_order: index as i32,
            })?;
        }
        Ok(())
    }

    fn on_render_overlay(&mut self, event: &RenderOverlayEvent) -> Result<(), PluginError> {
        if event.overlay_id.as_str() != OVERLAY_PIPES_ID {
            return Ok(());
        }
        let paused = self.time.is_paused().unwrap_or(true);
        let graph = with_bound_context("flux.default.render_overlay", |context| {
            let structures = context
                .structures
                .as_deref()
                .ok_or(PluginError::ApiUnavailable("flux.default.structures"))?;
            let pipe_gas = context
                .pipe_gas
                .as_deref()
                .ok_or(PluginError::ApiUnavailable("flux.default.pipe_gas"))?;
            let flow_state =
                context
                    .pipe_flow_visuals
                    .as_deref()
                    .ok_or(PluginError::ApiUnavailable(
                        "flux.default.pipe_flow_visuals",
                    ))?;
            let pipe_config = context
                .pipe_config
                .ok_or(PluginError::ApiUnavailable("flux.default.pipe_config"))?;
            let flow_progress = flow_state.flow_progress();
            Ok(build_pipes_overlay_graph(
                structures,
                pipe_gas,
                flow_state,
                pipe_config,
                context.gas_registry,
                flow_progress,
                paused,
            ))
        })?;
        self.overlays.submit_graph(graph)
    }
}

fn reset_runtime_state(clear_pipe_gas: bool) -> Result<(), PluginError> {
    with_bound_context("flux.default.reset_runtime_state", |context| {
        if clear_pipe_gas {
            if let Some(pipe_gas) = context.pipe_gas.as_deref_mut() {
                pipe_gas.clear_all();
            }
        }
        if let Some(pipe_flux) = context.pipe_flux.as_deref_mut() {
            pipe_flux.clear_all();
        }
        if let Some(flow_visuals) = context.pipe_flow_visuals.as_deref_mut() {
            flow_visuals.reset_flow();
        }
        Ok(())
    })
}

fn hud_block_id(index: usize) -> flux_plugin_sdk::ContentId {
    flux_plugin_sdk::ContentId::parse(&format!("flux.default.hud.block.{index}"))
        .expect("default HUD content ids must stay valid")
}

#[cfg(test)]
mod tests {
    use super::FluxDefaultRuntimeSdkPlugin;

    #[test]
    fn plugin_type_is_constructible() {
        let _ = std::mem::size_of::<FluxDefaultRuntimeSdkPlugin>();
    }
}
