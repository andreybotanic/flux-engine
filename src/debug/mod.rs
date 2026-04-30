use bevy::prelude::*;

use crate::{
    simulation::{
        do_one_substep,
        gas::{preview_next_substep, GasField},
        BlockSyncState, GasSimulationConfig, SimulationControl, SimulationStep,
    },
    world::grid::{cell_center, WorldGrid, CELL_SIZE},
};

#[derive(Resource, Default)]
pub struct DebugMode {
    pub active: bool,
}

#[derive(Resource, Clone, Copy)]
pub struct DebugOverlaySettings {
    pub show_diffusion_cells: bool,
}

impl Default for DebugOverlaySettings {
    fn default() -> Self {
        Self {
            show_diffusion_cells: true,
        }
    }
}

#[derive(Resource, Default)]
pub struct DebugStepPreview {
    /// Each entry: (from_x, from_y, to_x, to_y)
    pub moves: Vec<(u32, u32, u32, u32)>,
}

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugMode>()
            .init_resource::<DebugOverlaySettings>()
            .init_resource::<DebugStepPreview>()
            .add_systems(
                Update,
                (handle_debug_keys, update_debug_preview, draw_debug_overlays).chain(),
            );
    }
}

fn handle_debug_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut debug_mode: ResMut<DebugMode>,
    mut control: ResMut<SimulationControl>,
    config: Res<GasSimulationConfig>,
    mut block_state: ResMut<BlockSyncState>,
    mut gas: ResMut<GasField>,
    world: Res<WorldGrid>,
    mut step: ResMut<SimulationStep>,
) {
    if keys.just_pressed(KeyCode::Backquote) {
        debug_mode.active = !debug_mode.active;
        if debug_mode.active {
            control.paused = true;
        }
    }

    if debug_mode.active && keys.just_pressed(KeyCode::Enter) {
        do_one_substep(&mut block_state, &mut gas, &world, &config, &mut step);
    }
}

fn update_debug_preview(
    debug_mode: Res<DebugMode>,
    overlay_settings: Res<DebugOverlaySettings>,
    config: Res<GasSimulationConfig>,
    gas: Res<GasField>,
    world: Res<WorldGrid>,
    block_state: Res<BlockSyncState>,
    mut preview: ResMut<DebugStepPreview>,
) {
    if !debug_mode.active || !config.enable_diffusion || !overlay_settings.show_diffusion_cells {
        if !preview.moves.is_empty() {
            preview.moves.clear();
        }
        return;
    }

    // Recompute only when something relevant changed
    if !debug_mode.is_changed()
        && !overlay_settings.is_changed()
        && !gas.is_changed()
        && !block_state.is_changed()
    {
        return;
    }

    let (_, moves) = preview_next_substep(
        &gas,
        &world,
        block_state.rng_state,
        block_state.phase,
        [config.diffusion_k_h2, config.diffusion_k_o2],
        config.max_flux_fraction,
    );
    preview.moves = moves;
}

fn draw_debug_overlays(
    debug_mode: Res<DebugMode>,
    overlay_settings: Res<DebugOverlaySettings>,
    preview: Res<DebugStepPreview>,
    mut gizmos: Gizmos,
) {
    if !debug_mode.active || !overlay_settings.show_diffusion_cells {
        return;
    }

    let cell_size = Vec2::splat(CELL_SIZE - 1.0);

    for &(fx, fy, tx, ty) in &preview.moves {
        let from_center = cell_center(fx, fy);
        let to_center = cell_center(tx, ty);

        // Green outline: cell that will emit particles
        gizmos.rect_2d(
            Isometry2d::from_translation(from_center),
            cell_size,
            Color::srgba(0.15, 1.0, 0.15, 0.9),
        );

        // Blue outline: neighbor that will receive particles
        gizmos.rect_2d(
            Isometry2d::from_translation(to_center),
            cell_size,
            Color::srgba(0.15, 0.55, 1.0, 0.9),
        );
    }
}
