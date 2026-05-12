use bevy::{prelude::*, window::PrimaryWindow};

use super::cell_inspector_model::CellInspectorBlockView;
use crate::{
    config::{
        CellVisualPlacementConfigMap, GasRegistry, StructureHudConfigMap, StructureVisualConfigMap,
        WorldCellHudConfig,
    },
    editor::{is_cursor_over_ui, ActiveEditorTool, MainMenuState},
    input::camera::MainCamera,
    plugins::{
        default_plugin::pipe_runtime::{PipeFlowVisualState, PipeGasField, PipeSimulationConfig},
        ContentRegistry, PluginHudBlockStore, PluginRuntimeEvent, PluginRuntimeRegistry,
        RuntimeDllPluginRegistry, RuntimeHostContext,
    },
    save::WorldLoadState,
    simulation::gas::GasField,
    ui::palette,
    ui::panels::PanelManager,
    world::{
        grid::{world_to_cell, WorldGrid},
        structures::PlacedStructureMap,
    },
};

#[derive(Component)]
pub(crate) struct CellInspectorRoot;

#[derive(Component)]
pub(crate) struct CellInspectorBlockSlot {
    index: usize,
}

#[derive(Component)]
pub(crate) struct CellInspectorBlockTitle {
    index: usize,
}

#[derive(Component)]
pub(crate) struct CellInspectorBlockBody {
    index: usize,
}

const CELL_INSPECTOR_WIDTH: f32 = 320.0;
const CELL_INSPECTOR_MIN_HEIGHT: f32 = 108.0;
const CELL_INSPECTOR_LINE_HEIGHT: f32 = 16.0;
const CELL_INSPECTOR_ROOT_PADDING: f32 = 6.0;
const CELL_INSPECTOR_BLOCK_PADDING: f32 = 8.0;
const CELL_INSPECTOR_BLOCK_GAP: f32 = 6.0;
const CELL_INSPECTOR_BLOCK_CONTENT_GAP: f32 = 3.0;
const CELL_INSPECTOR_BLOCK_RADIUS: f32 = 8.0;
const CELL_INSPECTOR_ROOT_RADIUS: f32 = 10.0;
const CELL_INSPECTOR_BLOCK_SLOT_COUNT: usize = 8;

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
                padding: UiRect::all(Val::Px(CELL_INSPECTOR_ROOT_PADDING)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(CELL_INSPECTOR_BLOCK_GAP),
                ..default()
            },
            BackgroundColor(palette::TRANSPARENT),
            BorderRadius::all(Val::Px(CELL_INSPECTOR_ROOT_RADIUS)),
            BoxShadow::new(
                palette::HUD_SHADOW,
                Val::Px(0.0),
                Val::Px(5.0),
                Val::Px(0.0),
                Val::Px(12.0),
            ),
            Visibility::Hidden,
            CellInspectorRoot,
        ))
        .with_children(|parent| {
            for index in 0..CELL_INSPECTOR_BLOCK_SLOT_COUNT {
                spawn_cell_inspector_block_slot(parent, index);
            }
        });
}

/// Runs `update_cell_inspector` logic.
pub(crate) fn update_cell_inspector(
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    ui_state: (
        Res<ActiveEditorTool>,
        Res<MainMenuState>,
        Res<WorldLoadState>,
        Res<crate::debug::DebugMode>,
        Res<PanelManager>,
    ),
    gas_state: (
        Res<PipeSimulationConfig>,
        Res<GasRegistry>,
        Res<WorldCellHudConfig>,
        Res<CellVisualPlacementConfigMap>,
        Res<StructureHudConfigMap>,
        Res<StructureVisualConfigMap>,
        ResMut<GasField>,
        ResMut<WorldGrid>,
    ),
    pipe_state: (
        ResMut<PlacedStructureMap>,
        ResMut<PipeGasField>,
        ResMut<PipeFlowVisualState>,
    ),
    mut plugin_runtime: ResMut<RuntimeDllPluginRegistry>,
    plugin_registry: Res<PluginRuntimeRegistry>,
    content_registry: Res<ContentRegistry>,
    mut plugin_hud: ResMut<PluginHudBlockStore>,
    mut ui_queries: ParamSet<(
        Single<(&mut Node, &mut Visibility), With<CellInspectorRoot>>,
        Query<(&CellInspectorBlockSlot, &mut Node)>,
        Query<(&CellInspectorBlockTitle, &mut Text)>,
        Query<(&CellInspectorBlockBody, &mut Node, &mut Text)>,
    )>,
) {
    let (active_tool, main_menu, world_load_state, debug_mode, panel_manager) = ui_state;
    let (
        pipe_config,
        gas_registry,
        world_cell_hud,
        cell_visual_layouts,
        structure_hud,
        structure_visuals,
        mut gas,
        mut world,
    ) = gas_state;
    let (mut structures, mut pipe_gas, mut flow_state) = pipe_state;
    let (camera, camera_transform) = *camera_query;
    if !world_load_state.has_world {
        let (_, visibility) = &mut *ui_queries.p0();
        **visibility = Visibility::Hidden;
        return;
    }

    let Some(cursor_position) = window.cursor_position() else {
        let (_, visibility) = &mut *ui_queries.p0();
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
        let (_, visibility) = &mut *ui_queries.p0();
        **visibility = Visibility::Hidden;
        return;
    }

    let Ok(cursor_world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_position)
    else {
        let (_, visibility) = &mut *ui_queries.p0();
        **visibility = Visibility::Hidden;
        return;
    };
    let Some(cell) = world_to_cell(cursor_world_pos) else {
        let (_, visibility) = &mut *ui_queries.p0();
        **visibility = Visibility::Hidden;
        return;
    };

    plugin_hud.clear();
    {
        let mut plugin_context = RuntimeHostContext::new(&content_registry, &gas_registry);
        plugin_context.world = Some(&mut world);
        plugin_context.structures = Some(&mut structures);
        plugin_context.gas = Some(&mut gas);
        plugin_context.hud_blocks = Some(&mut plugin_hud);
        plugin_context.pipe_config = Some(&pipe_config);
        plugin_context.pipe_gas = Some(&mut pipe_gas);
        plugin_context.pipe_flow_visuals = Some(&mut flow_state);
        plugin_context.world_cell_hud = Some(&world_cell_hud);
        plugin_context.cell_visual_layouts = Some(&cell_visual_layouts);
        plugin_context.structure_hud = Some(&structure_hud);
        plugin_context.structure_visuals = Some(&structure_visuals);
        plugin_runtime.dispatch_event(
            &plugin_registry,
            &PluginRuntimeEvent::BuildHudForCell { cell },
            &mut plugin_context,
        );
    }
    let blocks = plugin_hud
        .blocks
        .iter()
        .map(|block| CellInspectorBlockView {
            title: block.title.clone(),
            lines: block.lines.clone(),
        })
        .collect::<Vec<_>>();
    let panel_height = estimate_cell_inspector_height(&blocks);
    let panel_position = compute_hud_position(
        cursor_position,
        Vec2::new(CELL_INSPECTOR_WIDTH, panel_height),
        Vec2::new(window.width(), window.height()),
        Vec2::new(24.0, 18.0),
    );

    {
        let (node, visibility) = &mut *ui_queries.p0();
        **visibility = Visibility::Visible;
        node.min_height = Val::Px(panel_height);
        node.left = Val::Px(panel_position.x);
        node.top = Val::Px(panel_position.y);
    }

    sync_cell_inspector_slots(&blocks, &mut ui_queries);
}

fn spawn_cell_inspector_block_slot(parent: &mut ChildSpawnerCommands, index: usize) {
    parent
        .spawn((
            Node {
                display: Display::None,
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(CELL_INSPECTOR_BLOCK_PADDING)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(CELL_INSPECTOR_BLOCK_CONTENT_GAP),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(palette::HUD_BG),
            BorderColor(palette::HUD_BORDER),
            BorderRadius::all(Val::Px(CELL_INSPECTOR_BLOCK_RADIUS)),
            CellInspectorBlockSlot { index },
        ))
        .with_children(|block_parent| {
            block_parent.spawn((
                Text::new(""),
                TextFont::from_font_size(13.0),
                TextColor(palette::TEXT_ON_DARK),
                CellInspectorBlockTitle { index },
            ));
            block_parent.spawn((
                Node {
                    display: Display::None,
                    ..default()
                },
                Text::new(""),
                TextFont::from_font_size(12.0),
                TextColor(palette::HUD_TEXT_MUTED),
                CellInspectorBlockBody { index },
            ));
        });
}

fn sync_cell_inspector_slots(
    blocks: &[CellInspectorBlockView],
    ui_queries: &mut ParamSet<(
        Single<(&mut Node, &mut Visibility), With<CellInspectorRoot>>,
        Query<(&CellInspectorBlockSlot, &mut Node)>,
        Query<(&CellInspectorBlockTitle, &mut Text)>,
        Query<(&CellInspectorBlockBody, &mut Node, &mut Text)>,
    )>,
) {
    debug_assert!(
        blocks.len() <= CELL_INSPECTOR_BLOCK_SLOT_COUNT,
        "cell inspector requires more slots than preallocated"
    );

    {
        let mut block_query = ui_queries.p1();
        for (slot, mut node) in block_query.iter_mut() {
            node.display = if blocks.get(slot.index).is_some() {
                Display::Flex
            } else {
                Display::None
            };
        }
    }

    {
        let mut title_query = ui_queries.p2();
        for (title, mut text) in title_query.iter_mut() {
            text.0 = blocks
                .get(title.index)
                .map(|block| block.title.clone())
                .unwrap_or_default();
        }
    }

    {
        let mut body_query = ui_queries.p3();
        for (body, mut node, mut text) in body_query.iter_mut() {
            if let Some(block) = blocks.get(body.index) {
                if block.lines.is_empty() {
                    node.display = Display::None;
                    text.0.clear();
                } else {
                    node.display = Display::Flex;
                    text.0 = block.lines.join("\n");
                }
            } else {
                node.display = Display::None;
                text.0.clear();
            }
        }
    }
}

fn estimate_cell_inspector_height(blocks: &[CellInspectorBlockView]) -> f32 {
    let block_heights = blocks
        .iter()
        .map(estimate_cell_inspector_block_height)
        .sum::<f32>();
    let gaps = blocks.len().saturating_sub(1) as f32 * CELL_INSPECTOR_BLOCK_GAP;
    (block_heights + gaps + CELL_INSPECTOR_ROOT_PADDING * 2.0).max(CELL_INSPECTOR_MIN_HEIGHT)
}

fn estimate_cell_inspector_block_height(block: &CellInspectorBlockView) -> f32 {
    let line_count = 1 + block.lines.len();
    let gap = if block.lines.is_empty() {
        0.0
    } else {
        CELL_INSPECTOR_BLOCK_CONTENT_GAP
    };
    line_count as f32 * CELL_INSPECTOR_LINE_HEIGHT + gap + CELL_INSPECTOR_BLOCK_PADDING * 2.0
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
    use super::{compute_hud_position, estimate_cell_inspector_height, CellInspectorBlockView};
    use bevy::prelude::*;

    #[test]
    fn hud_flips_to_left_and_up_near_screen_edge() {
        let pos = compute_hud_position(
            Vec2::new(1590.0, 890.0),
            Vec2::new(320.0, 108.0),
            Vec2::new(1600.0, 900.0),
            Vec2::new(24.0, 18.0),
        );
        assert!(pos.x < 1590.0);
        assert!(pos.y < 890.0);
    }

    #[test]
    fn panel_height_grows_with_more_blocks_and_lines() {
        let short = vec![CellInspectorBlockView {
            title: "Cell".to_string(),
            lines: vec!["Particles: 0".to_string()],
        }];
        let tall = vec![
            CellInspectorBlockView {
                title: "Cell".to_string(),
                lines: vec![
                    "Coordinates: (1, 1)".to_string(),
                    "State: Empty".to_string(),
                    "Pressure: 0Pa".to_string(),
                    "Particles: 0".to_string(),
                ],
            },
            CellInspectorBlockView {
                title: "Pipe".to_string(),
                lines: vec![
                    "Pressure: 250Pa".to_string(),
                    "Particles: 10".to_string(),
                    "Gases: Hydrogen 100%".to_string(),
                ],
            },
        ];
        assert!(estimate_cell_inspector_height(&tall) > estimate_cell_inspector_height(&short));
    }
}
