use bevy::{prelude::*, window::PrimaryWindow};

use crate::{
    config::GasRegistry,
    editor::{is_cursor_over_ui, ActiveEditorTool, MainMenuState},
    input::camera::MainCamera,
    render::OverlayMode,
    save::WorldLoadState,
    simulation::{
        gas::GasField,
        pipes::{
            pipe_cell_display_blocks_with_transfers,
            pressure::{format_pressure_pa, pipe_pressure_pa, world_pressure_pa},
            PipeContainerKind, PipeFlowVisualState, PipeGasField,
        },
        GasSimulationConfig, SimulationControl,
    },
    ui::palette,
    ui::panels::PanelManager,
    world::{
        grid::{world_to_cell, WorldGrid},
        structures::PlacedStructureMap,
    },
};

#[derive(Component)]
pub(crate) struct CellInspectorText;

#[derive(Component)]
pub(crate) struct CellInspectorPanel;

const CELL_INSPECTOR_WIDTH: f32 = 310.0;
const CELL_INSPECTOR_MIN_HEIGHT: f32 = 108.0;
const CELL_INSPECTOR_LINE_HEIGHT: f32 = 16.0;
const CELL_INSPECTOR_VERTICAL_PADDING: f32 = 18.0;

/// Runs `setup_cell_inspector` logic.
pub(crate) fn setup_cell_inspector(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                left: Val::Px(12.0),
                width: Val::Px(CELL_INSPECTOR_WIDTH),
                min_height: Val::Px(CELL_INSPECTOR_MIN_HEIGHT),
                padding: UiRect::all(Val::Px(7.0)),
                ..default()
            },
            BackgroundColor(palette::HUD_BG),
            CellInspectorPanel,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("FluxEngine loading..."),
                TextFont::from_font_size(13.0),
                TextColor(palette::TEXT_ON_DARK),
                CellInspectorText,
            ));
        });
}

/// Runs `update_cell_inspector` logic.
pub(crate) fn update_cell_inspector(
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    active_tool: Res<ActiveEditorTool>,
    main_menu: Res<MainMenuState>,
    world_load_state: Res<WorldLoadState>,
    debug_mode: Res<crate::debug::DebugMode>,
    panel_manager: Res<PanelManager>,
    gas: Res<GasField>,
    config: Res<GasSimulationConfig>,
    gas_registry: Res<GasRegistry>,
    world: Res<WorldGrid>,
    pipe_state: (
        Res<PlacedStructureMap>,
        Res<PipeGasField>,
        Res<PipeFlowVisualState>,
        Res<SimulationControl>,
    ),
    overlay_mode: Res<OverlayMode>,
    mut text_query: Single<&mut Text, With<CellInspectorText>>,
    mut panel_query: Single<(&mut Node, &mut Visibility), With<CellInspectorPanel>>,
) {
    let (camera, camera_transform) = *camera_query;
    let text = &mut *text_query;
    let (node, visibility) = &mut *panel_query;
    let (pipe_layout, pipe_gas, flow_state, _control) = pipe_state;

    if !world_load_state.has_world {
        **visibility = Visibility::Hidden;
        return;
    }

    let overlay_name = match *overlay_mode {
        OverlayMode::Main => "F1 Main",
        OverlayMode::Gas => "F2 Gas",
        OverlayMode::Pipes => "F3 Pipes",
    };

    let Some(cursor_position) = window.cursor_position() else {
        **visibility = Visibility::Hidden;
        return;
    };

    if is_cursor_over_ui(
        cursor_position,
        &window,
        debug_mode.active,
        active_tool.selected,
        main_menu.open,
        Some(&panel_manager),
    ) {
        **visibility = Visibility::Hidden;
        return;
    }

    let Ok(cursor_world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_position)
    else {
        **visibility = Visibility::Hidden;
        return;
    };
    let Some(cell) = world_to_cell(cursor_world_pos) else {
        **visibility = Visibility::Hidden;
        return;
    };
    **visibility = Visibility::Visible;

    let message = build_cell_inspector_message(
        overlay_name,
        cell,
        &world,
        &gas,
        &config.pipe,
        &gas_registry,
        &pipe_layout,
        &pipe_gas,
        &flow_state,
        false,
    );
    let panel_height = estimate_cell_inspector_height(&message);
    node.min_height = Val::Px(panel_height);

    if let Some(cursor_position) = window.cursor_position() {
        let panel_size = Vec2::new(CELL_INSPECTOR_WIDTH, panel_height);
        let panel_position = compute_hud_position(
            cursor_position,
            panel_size,
            Vec2::new(window.width(), window.height()),
            Vec2::new(24.0, 18.0),
        );
        node.left = Val::Px(panel_position.x);
        node.top = Val::Px(panel_position.y);
    }

    text.0 = message;
}

fn build_cell_inspector_message(
    overlay_name: &str,
    cell: UVec2,
    world: &WorldGrid,
    gas: &GasField,
    pipe_config: &crate::simulation::PipeSimulationConfig,
    gas_registry: &GasRegistry,
    structures: &PlacedStructureMap,
    pipe_gas: &PipeGasField,
    flow_state: &PipeFlowVisualState,
    include_transfers: bool,
) -> String {
    let cell_kind = if world.is_solid(cell.x, cell.y) {
        "solid"
    } else {
        "empty"
    };
    let mut message = format!("{overlay_name}\n({}, {}) {}\n", cell.x, cell.y, cell_kind);
    append_gas_lines(&mut message, gas_registry, |gas_index| {
        gas.amount_rounded(cell.x, cell.y, gas_index)
    });
    message.push_str(&format!(
        "Total: {} particles",
        gas.total_amount_rounded(cell.x, cell.y)
    ));
    message.push_str(&format!(
        "\nPressure: {}",
        format_pressure_pa(world_pressure_pa(
            pipe_config,
            gas.total_amount_rounded(cell.x, cell.y),
        ))
    ));

    let display_blocks = pipe_cell_display_blocks_with_transfers(
        structures,
        pipe_gas,
        flow_state,
        cell.x,
        cell.y,
        include_transfers,
    );
    if !display_blocks.is_empty() {
        message.push_str("\n\n");
        for (index, block) in display_blocks.iter().enumerate() {
            if index > 0 {
                message.push('\n');
            }
            let label = match block.kind {
                PipeContainerKind::Pipe if structures.has_vent_at(cell.x, cell.y) => "Pipe + Vent",
                PipeContainerKind::Pipe => "Pipe",
                PipeContainerKind::BridgePipe => "Bridge Pipe",
            };
            message.push_str(label);
            message.push('\n');
            append_gas_lines(&mut message, gas_registry, |gas_index| {
                block.species_counts.get(gas_index).copied().unwrap_or(0)
            });
            message.push_str(&format!(
                "{} total: {} particles",
                label, block.total_particles
            ));
            message.push_str(&format!(
                "\n{} pressure: {}",
                label,
                format_pressure_pa(pipe_pressure_pa(pipe_config, block.total_particles))
            ));
            if index + 1 < display_blocks.len() {
                message.push_str("\n\n");
            }
        }
    }

    message
}

fn append_gas_lines<F>(message: &mut String, gas_registry: &GasRegistry, mut amount_for: F)
where
    F: FnMut(usize) -> u32,
{
    for (gas_index, gas_def) in gas_registry.all().iter().enumerate() {
        message.push_str(&format!(
            "{}: {} particles\n",
            gas_def.id.to_uppercase(),
            amount_for(gas_index)
        ));
    }
}

fn estimate_cell_inspector_height(message: &str) -> f32 {
    let line_count = message.lines().count().max(1) as f32;
    (line_count * CELL_INSPECTOR_LINE_HEIGHT + CELL_INSPECTOR_VERTICAL_PADDING)
        .max(CELL_INSPECTOR_MIN_HEIGHT)
}

fn compute_hud_position(cursor: Vec2, panel_size: Vec2, viewport_size: Vec2, offset: Vec2) -> Vec2 {
    let mut x = cursor.x + offset.x;
    let mut y = cursor.y + offset.y;
    if x + panel_size.x > viewport_size.x {
        x = cursor.x - panel_size.x - offset.x;
    }
    if y + panel_size.y > viewport_size.y {
        y = cursor.y - panel_size.y - offset.y;
    }
    Vec2::new(
        x.clamp(0.0, (viewport_size.x - panel_size.x).max(0.0)),
        y.clamp(0.0, (viewport_size.y - panel_size.y).max(0.0)),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        build_cell_inspector_message, compute_hud_position, estimate_cell_inspector_height,
    };
    use crate::{
        config::{GasDefinition, GasRegistry},
        simulation::{
            gas::GasField,
            pipes::{
                pressure::{format_pressure_pa, pipe_pressure_pa, world_pressure_pa},
                PipeFlowVisualState, PipeGasField, PipeTransferRecord, PipeTransferVisualPath,
            },
            PipeSimulationConfig,
        },
        world::{grid::WorldGrid, structures::PlacedStructureMap},
    };
    use bevy::prelude::*;

    fn registry() -> GasRegistry {
        GasRegistry::new(vec![
            GasDefinition {
                id: "h2".to_string(),
                label: "Hydrogen".to_string(),
                color: [0.7, 0.8, 1.0],
                molecular_mass: 2.016,
            },
            GasDefinition {
                id: "o2".to_string(),
                label: "Oxygen".to_string(),
                color: [0.5, 0.8, 1.0],
                molecular_mass: 31.998,
            },
        ])
        .expect("test registry")
    }

    fn pipe_config() -> PipeSimulationConfig {
        PipeSimulationConfig::default()
    }

    #[test]
    fn hud_flips_to_left_and_up_near_screen_edge() {
        let pos = compute_hud_position(
            Vec2::new(1590.0, 890.0),
            Vec2::new(310.0, 108.0),
            Vec2::new(1600.0, 900.0),
            Vec2::new(24.0, 18.0),
        );
        assert!(pos.x < 1590.0);
        assert!(pos.y < 890.0);
    }

    #[test]
    fn pipe_block_is_added_when_hovered_cell_contains_pipe() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        let flow_state = PipeFlowVisualState::default();
        let mut gas = GasField::from_registry(&registry);
        assert!(structures.place_pipe(12, 14, &world));
        assert!(structures.place_vent(12, 14, &world));
        gas.set_amount(12, 14, 0, 5.0);
        pipe_gas.sync_to_structures(&structures);
        pipe_gas.add_species_counts(0, &[7, 2]);

        let message = build_cell_inspector_message(
            "F3 Pipes",
            UVec2::new(12, 14),
            &world,
            &gas,
            &pipe_config(),
            &registry,
            &structures,
            &pipe_gas,
            &flow_state,
            true,
        );

        assert!(message.contains("Pipe + Vent"));
        assert!(message.contains("Pipe + Vent total: 9 particles"));
        assert!(message.contains(&format!(
            "Pipe + Vent pressure: {}",
            format_pressure_pa(pipe_pressure_pa(&pipe_config(), 9))
        )));
        assert!(message.contains("H2: 7 particles"));
        assert!(message.contains("O2: 2 particles"));
    }

    #[test]
    fn panel_height_grows_when_pipe_block_is_present() {
        let short = "F1 Main\n(1, 1) empty\nH2: 0 particles\nTotal: 0 particles\nPressure: 0Pa";
        let tall = "F3 Pipes\n(1, 1) empty\nH2: 0 particles\nTotal: 0 particles\nPressure: 0Pa\n\nPipe\nH2: 10 particles\nPipe total: 10 particles\nPipe pressure: 250Pa";
        assert!(estimate_cell_inspector_height(tall) > estimate_cell_inspector_height(short));
    }

    #[test]
    fn pipe_block_uses_transient_flow_when_storage_is_empty() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        let pipe_gas = PipeGasField::from_registry(&registry);
        let gas = GasField::from_registry(&registry);
        let flow_state = PipeFlowVisualState {
            transfers: vec![PipeTransferRecord {
                from: UVec2::new(8, 9),
                to: UVec2::new(9, 9),
                gas_counts: vec![4, 1],
                total_amount: 5,
                visual_path: PipeTransferVisualPath::Straight,
            }],
        };
        assert!(structures.place_pipe(8, 9, &world));
        let mut pipe_gas = pipe_gas;
        pipe_gas.sync_to_structures(&structures);

        let message = build_cell_inspector_message(
            "F3 Pipes",
            UVec2::new(8, 9),
            &world,
            &gas,
            &pipe_config(),
            &registry,
            &structures,
            &pipe_gas,
            &flow_state,
            true,
        );

        assert!(message.contains("H2: 4 particles"));
        assert!(message.contains("O2: 1 particles"));
        assert!(message.contains("Pipe total: 5 particles"));
    }

    #[test]
    fn pipe_block_ignores_transient_flow_when_requested() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        let pipe_gas = PipeGasField::from_registry(&registry);
        let gas = GasField::from_registry(&registry);
        let flow_state = PipeFlowVisualState {
            transfers: vec![PipeTransferRecord {
                from: UVec2::new(8, 9),
                to: UVec2::new(9, 9),
                gas_counts: vec![4, 1],
                total_amount: 5,
                visual_path: PipeTransferVisualPath::Straight,
            }],
        };
        assert!(structures.place_pipe(8, 9, &world));
        let mut pipe_gas = pipe_gas;
        pipe_gas.sync_to_structures(&structures);

        let message = build_cell_inspector_message(
            "F3 Pipes",
            UVec2::new(8, 9),
            &world,
            &gas,
            &pipe_config(),
            &registry,
            &structures,
            &pipe_gas,
            &flow_state,
            false,
        );

        assert!(message.contains("H2: 0 particles"));
        assert!(message.contains("O2: 0 particles"));
        assert!(message.contains("Pipe total: 0 particles"));
    }

    #[test]
    fn message_shows_world_pressure_line() {
        let registry = registry();
        let world = WorldGrid::default();
        let structures = PlacedStructureMap::default();
        let pipe_gas = PipeGasField::from_registry(&registry);
        let mut gas = GasField::from_registry(&registry);
        gas.set_amount(4, 5, 0, 1_250.0);

        let message = build_cell_inspector_message(
            "F1 Main",
            UVec2::new(4, 5),
            &world,
            &gas,
            &pipe_config(),
            &registry,
            &structures,
            &pipe_gas,
            &PipeFlowVisualState::default(),
            false,
        );

        assert!(message.contains("Total: 1250 particles"));
        assert!(message.contains(&format!(
            "Pressure: {}",
            format_pressure_pa(world_pressure_pa(&pipe_config(), 1_250))
        )));
    }
}
