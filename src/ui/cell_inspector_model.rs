use bevy::prelude::*;

use crate::{
    config::{
        CellVisualPlacementConfigMap, ConfiguredPipeNodeKind, ContainerBacking, GasRegistry,
        HoverVisibility, StructureHudConfigMap, StructureVisualConfigMap, SubstanceContainerConfig,
        SubstanceKind, WorldCellHudConfig,
    },
    plugins::default_plugin::pipe_runtime::{
        pipe_cell_display_blocks_with_transfers,
        pressure::{format_pressure_pa, pipe_pressure_pa, world_pressure_pa},
        PipeSimulationConfig,
    },
    plugins::default_plugin::pipe_runtime::{PipeContainerKind, PipeFlowVisualState, PipeGasField},
    simulation::gas::GasField,
    world::{
        grid::{CellMaterial, WorldGrid},
        structures::{bridge_center_cell, PlacedStructure, PlacedStructureMap},
    },
};

/// Stores one rendered HUD block for the cell inspector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CellInspectorBlockView {
    pub title: String,
    pub lines: Vec<String>,
}

/// Builds every HUD block that should be shown for the hovered cell.
pub(crate) fn build_cell_inspector_blocks(
    cell: UVec2,
    world: &WorldGrid,
    gas: &GasField,
    pipe_config: &PipeSimulationConfig,
    gas_registry: &GasRegistry,
    world_cell_hud: &WorldCellHudConfig,
    cell_visual_layouts: &CellVisualPlacementConfigMap,
    structure_hud: &StructureHudConfigMap,
    structure_visuals: &StructureVisualConfigMap,
    structures: &PlacedStructureMap,
    pipe_gas: &PipeGasField,
    flow_state: &PipeFlowVisualState,
    include_transfers: bool,
) -> Vec<CellInspectorBlockView> {
    let mut blocks = vec![build_world_cell_block(
        cell,
        gas,
        pipe_config,
        gas_registry,
        world_cell_hud,
    )];
    if let Some(material) = world.solid_material(cell.x, cell.y) {
        blocks.push(build_solid_material_block(material, cell_visual_layouts));
    }

    let mut structure_list = structures.structures_at(cell.x, cell.y);
    structure_list
        .sort_by_key(|structure| (structure_hud.get(structure.kind).sort_order, structure.id));

    for structure in structure_list {
        blocks.push(build_structure_block(
            cell,
            structure,
            gas,
            pipe_config,
            gas_registry,
            structure_hud,
            structure_visuals,
            structures,
            pipe_gas,
            flow_state,
            include_transfers,
        ));
    }

    blocks
}

fn build_world_cell_block(
    cell: UVec2,
    gas: &GasField,
    pipe_config: &PipeSimulationConfig,
    gas_registry: &GasRegistry,
    world_cell_hud: &WorldCellHudConfig,
) -> CellInspectorBlockView {
    let mut lines = vec![format!("Coordinates: ({}, {})", cell.x, cell.y)];
    lines.extend(build_world_cell_container_lines(
        cell,
        gas,
        pipe_config,
        gas_registry,
        &world_cell_hud.block.substance_containers,
    ));

    CellInspectorBlockView {
        title: world_cell_hud.label.clone(),
        lines,
    }
}

fn build_solid_material_block(
    material: CellMaterial,
    cell_visual_layouts: &CellVisualPlacementConfigMap,
) -> CellInspectorBlockView {
    CellInspectorBlockView {
        title: cell_visual_layouts.get(material).label.clone(),
        lines: Vec::new(),
    }
}

fn build_structure_block(
    hovered_cell: UVec2,
    structure: &PlacedStructure,
    gas: &GasField,
    pipe_config: &PipeSimulationConfig,
    gas_registry: &GasRegistry,
    structure_hud: &StructureHudConfigMap,
    structure_visuals: &StructureVisualConfigMap,
    structures: &PlacedStructureMap,
    pipe_gas: &PipeGasField,
    flow_state: &PipeFlowVisualState,
    include_transfers: bool,
) -> CellInspectorBlockView {
    let hud = structure_hud.get(structure.kind);
    let lines = build_structure_container_lines(
        hovered_cell,
        structure,
        gas,
        pipe_config,
        gas_registry,
        structures,
        pipe_gas,
        flow_state,
        &hud.substance_containers,
        include_transfers,
    );

    CellInspectorBlockView {
        title: structure_visuals.get(structure.kind).label.clone(),
        lines,
    }
}

fn build_world_cell_container_lines(
    cell: UVec2,
    gas: &GasField,
    pipe_config: &PipeSimulationConfig,
    gas_registry: &GasRegistry,
    containers: &[SubstanceContainerConfig],
) -> Vec<String> {
    let mut lines = Vec::new();
    for (index, container) in containers.iter().enumerate() {
        let Some(section_lines) =
            build_world_cell_substance_lines(cell, gas, pipe_config, gas_registry, container)
        else {
            continue;
        };
        if index > 0 && !lines.is_empty() {
            lines.push(String::new());
        }
        lines.extend(section_lines);
    }
    lines
}

fn build_structure_container_lines(
    hovered_cell: UVec2,
    structure: &PlacedStructure,
    gas: &GasField,
    pipe_config: &PipeSimulationConfig,
    gas_registry: &GasRegistry,
    structures: &PlacedStructureMap,
    pipe_gas: &PipeGasField,
    flow_state: &PipeFlowVisualState,
    containers: &[SubstanceContainerConfig],
    include_transfers: bool,
) -> Vec<String> {
    let mut sections = Vec::new();
    for container in containers {
        let Some(section_lines) = build_structure_substance_lines(
            hovered_cell,
            structure,
            gas,
            pipe_config,
            gas_registry,
            structures,
            pipe_gas,
            flow_state,
            container,
            include_transfers,
        ) else {
            continue;
        };
        sections.push((container.substance, section_lines));
    }

    if sections.is_empty() {
        return Vec::new();
    }

    let multi_section = sections.len() > 1;
    let mut lines = Vec::new();
    for (index, (substance, section_lines)) in sections.into_iter().enumerate() {
        if index > 0 {
            lines.push(String::new());
        }
        if multi_section {
            lines.push(format!("{}:", substance_heading(substance)));
        }
        lines.extend(section_lines);
    }
    lines
}

fn build_world_cell_substance_lines(
    cell: UVec2,
    gas: &GasField,
    pipe_config: &PipeSimulationConfig,
    gas_registry: &GasRegistry,
    container: &SubstanceContainerConfig,
) -> Option<Vec<String>> {
    match (container.substance, container.backing) {
        (SubstanceKind::Gas, ContainerBacking::WorldCell) => {
            let species_counts = gas_species_counts_for_world_cell(cell, gas, gas_registry);
            let total_particles = gas.total_amount_rounded(cell.x, cell.y);
            Some(format_gas_lines(
                gas_registry,
                pipe_config,
                total_particles,
                &species_counts,
                false,
            ))
        }
        _ => None,
    }
}

fn build_structure_substance_lines(
    hovered_cell: UVec2,
    structure: &PlacedStructure,
    gas: &GasField,
    pipe_config: &PipeSimulationConfig,
    gas_registry: &GasRegistry,
    structures: &PlacedStructureMap,
    pipe_gas: &PipeGasField,
    flow_state: &PipeFlowVisualState,
    container: &SubstanceContainerConfig,
    include_transfers: bool,
) -> Option<Vec<String>> {
    match (container.substance, container.backing) {
        (SubstanceKind::Gas, ContainerBacking::WorldCell) => {
            let visible = hovered_cell_matches_visibility(
                container.visible_on_hover,
                structure,
                hovered_cell,
                Some(hovered_cell),
            );
            if !visible {
                return None;
            }
            let species_counts = gas_species_counts_for_world_cell(hovered_cell, gas, gas_registry);
            let total_particles = gas.total_amount_rounded(hovered_cell.x, hovered_cell.y);
            Some(format_gas_lines(
                gas_registry,
                pipe_config,
                total_particles,
                &species_counts,
                false,
            ))
        }
        (SubstanceKind::Gas, ContainerBacking::PipeNode { kind }) => {
            let container_cell = resolve_pipe_container_cell(kind, structure)?;
            if !hovered_cell_matches_visibility(
                container.visible_on_hover,
                structure,
                hovered_cell,
                Some(container_cell),
            ) {
                return None;
            }

            let expected_kind = match kind {
                ConfiguredPipeNodeKind::Pipe => PipeContainerKind::Pipe,
                ConfiguredPipeNodeKind::BridgePipe => PipeContainerKind::BridgePipe,
            };
            let display_block = pipe_cell_display_blocks_with_transfers(
                structures,
                pipe_gas,
                flow_state,
                container_cell.x,
                container_cell.y,
                include_transfers,
            )
            .into_iter()
            .find(|block| block.kind == expected_kind)?;

            Some(format_gas_lines(
                gas_registry,
                pipe_config,
                display_block.total_particles,
                &display_block.species_counts,
                true,
            ))
        }
    }
}

fn hovered_cell_matches_visibility(
    visibility: HoverVisibility,
    structure: &PlacedStructure,
    hovered_cell: UVec2,
    container_cell: Option<UVec2>,
) -> bool {
    match visibility {
        HoverVisibility::SameCell => structure.occupied_cells().contains(&hovered_cell),
        HoverVisibility::ContainerCell => container_cell == Some(hovered_cell),
    }
}

fn resolve_pipe_container_cell(
    kind: ConfiguredPipeNodeKind,
    structure: &PlacedStructure,
) -> Option<UVec2> {
    match kind {
        ConfiguredPipeNodeKind::Pipe => Some(structure.origin),
        ConfiguredPipeNodeKind::BridgePipe => {
            bridge_center_cell(structure.origin, structure.rotation)
        }
    }
}

fn gas_species_counts_for_world_cell(
    cell: UVec2,
    gas: &GasField,
    gas_registry: &GasRegistry,
) -> Vec<u32> {
    gas_registry
        .all()
        .iter()
        .enumerate()
        .map(|(gas_index, _)| gas.amount_rounded(cell.x, cell.y, gas_index))
        .collect()
}

fn format_gas_lines(
    gas_registry: &GasRegistry,
    pipe_config: &PipeSimulationConfig,
    total_particles: u32,
    species_counts: &[u32],
    is_pipe_container: bool,
) -> Vec<String> {
    let pressure = if is_pipe_container {
        pipe_pressure_pa(pipe_config, total_particles)
    } else {
        world_pressure_pa(pipe_config, total_particles)
    };

    let mut lines = vec![
        format!("Pressure: {}", format_pressure_pa(pressure)),
        format!("Particles: {}", total_particles),
    ];
    if let Some(composition) = format_gas_composition(gas_registry, species_counts) {
        lines.push(format!("Gases: {}", composition));
    }
    lines
}

fn format_gas_composition(gas_registry: &GasRegistry, species_counts: &[u32]) -> Option<String> {
    let total_particles: u32 = species_counts.iter().copied().sum();
    if total_particles == 0 {
        return None;
    }

    let mut parts = species_counts
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, amount)| *amount > 0)
        .filter_map(|(gas_index, amount)| {
            gas_registry.get(gas_index).map(|definition| {
                let ratio = amount as f64 / total_particles.max(1) as f64;
                (gas_index, definition.label.as_str(), amount, ratio)
            })
        })
        .collect::<Vec<_>>();
    parts.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));

    let dominant = parts.first().copied()?;
    let dominant_fragment = format!("{} {}", dominant.1, format_percentage_ratio(dominant.3));
    if dominant.3 > 0.9 {
        let impurities = parts
            .iter()
            .skip(1)
            .map(|(_, label, _, ratio)| format!("{} {}", label, format_percentage_ratio(*ratio)))
            .collect::<Vec<_>>();
        if impurities.is_empty() {
            Some(dominant_fragment)
        } else {
            Some(format!(
                "{} (impurities: {})",
                dominant_fragment,
                impurities.join(", ")
            ))
        }
    } else {
        Some(
            parts
                .iter()
                .map(|(_, label, _, ratio)| {
                    format!("{} {}", label, format_percentage_ratio(*ratio))
                })
                .collect::<Vec<_>>()
                .join(", "),
        )
    }
}

fn format_percentage_ratio(ratio: f64) -> String {
    let percent = ratio.clamp(0.0, 1.0) * 100.0;
    if percent < 0.1 {
        return "< 0.1%".to_string();
    }
    let rounded = (percent * 10.0).round() / 10.0;
    if (rounded - rounded.round()).abs() <= 1e-6 {
        format!("{}%", rounded.round() as i64)
    } else {
        format!("{rounded:.1}%")
    }
}

fn substance_heading(substance: SubstanceKind) -> &'static str {
    match substance {
        SubstanceKind::Gas => "Gas",
    }
}

#[cfg(test)]
mod tests {
    use super::{build_cell_inspector_blocks, format_percentage_ratio};
    use crate::{
        config::{
            CellVisualPlacementConfigMap, ConfiguredPipeNodeKind, ContainerBacking, GasDefinition,
            GasRegistry, HoverVisibility, HudBlockConfig, StructureHudConfigMap,
            StructureVisualConfigMap, SubstanceContainerConfig, SubstanceKind,
            VisualPlacementConfig, WorldCellHudConfig,
        },
        plugins::default_plugin::pipe_runtime::{
            PipeContainerKind, PipeFlowVisualState, PipeGasField, PipeSimulationConfig,
            PipeTransferRecord, PipeTransferVisualPath,
        },
        simulation::gas::GasField,
        world::{
            grid::WorldGrid,
            structures::{PlacedStructureMap, StructureRotation},
        },
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
            GasDefinition {
                id: "co2".to_string(),
                label: "Carbon dioxide".to_string(),
                color: [0.8, 0.8, 0.8],
                molecular_mass: 44.009,
            },
        ])
        .expect("test registry")
    }

    fn pipe_config() -> PipeSimulationConfig {
        PipeSimulationConfig::default()
    }

    fn world_hud_config() -> WorldCellHudConfig {
        WorldCellHudConfig {
            label: "Cell".to_string(),
            block: HudBlockConfig {
                sort_order: 0,
                substance_containers: vec![SubstanceContainerConfig {
                    substance: SubstanceKind::Gas,
                    backing: ContainerBacking::WorldCell,
                    visible_on_hover: HoverVisibility::SameCell,
                }],
            },
        }
    }

    fn structure_hud_config() -> StructureHudConfigMap {
        StructureHudConfigMap::from_entries(vec![
            (
                crate::plugins::default_plugin::pipe_structure_kind(),
                HudBlockConfig {
                    sort_order: 10,
                    substance_containers: vec![SubstanceContainerConfig {
                        substance: SubstanceKind::Gas,
                        backing: ContainerBacking::PipeNode {
                            kind: ConfiguredPipeNodeKind::Pipe,
                        },
                        visible_on_hover: HoverVisibility::SameCell,
                    }],
                },
            ),
            (
                crate::plugins::default_plugin::gas_pipe_bridge_structure_kind(),
                HudBlockConfig {
                    sort_order: 20,
                    substance_containers: vec![SubstanceContainerConfig {
                        substance: SubstanceKind::Gas,
                        backing: ContainerBacking::PipeNode {
                            kind: ConfiguredPipeNodeKind::BridgePipe,
                        },
                        visible_on_hover: HoverVisibility::ContainerCell,
                    }],
                },
            ),
            (
                crate::plugins::default_plugin::vent_structure_kind(),
                HudBlockConfig {
                    sort_order: 30,
                    substance_containers: Vec::new(),
                },
            ),
            (
                crate::plugins::default_plugin::gas_source_structure_kind(),
                HudBlockConfig {
                    sort_order: 40,
                    substance_containers: Vec::new(),
                },
            ),
            (
                crate::plugins::default_plugin::gas_sink_structure_kind(),
                HudBlockConfig {
                    sort_order: 50,
                    substance_containers: Vec::new(),
                },
            ),
        ])
    }

    fn structure_visuals() -> StructureVisualConfigMap {
        StructureVisualConfigMap::from_entries(vec![
            (
                crate::plugins::default_plugin::pipe_structure_kind(),
                VisualPlacementConfig {
                    label: "Pipe".to_string(),
                    draw_priority: 100,
                    size_in_cells: UVec2::ONE,
                },
            ),
            (
                crate::plugins::default_plugin::gas_pipe_bridge_structure_kind(),
                VisualPlacementConfig {
                    label: "Bridge".to_string(),
                    draw_priority: 110,
                    size_in_cells: UVec2::new(3, 1),
                },
            ),
            (
                crate::plugins::default_plugin::vent_structure_kind(),
                VisualPlacementConfig {
                    label: "Vent".to_string(),
                    draw_priority: 120,
                    size_in_cells: UVec2::ONE,
                },
            ),
            (
                crate::plugins::default_plugin::gas_source_structure_kind(),
                VisualPlacementConfig {
                    label: "Gas Source".to_string(),
                    draw_priority: 130,
                    size_in_cells: UVec2::ONE,
                },
            ),
            (
                crate::plugins::default_plugin::gas_sink_structure_kind(),
                VisualPlacementConfig {
                    label: "Gas Sink".to_string(),
                    draw_priority: 130,
                    size_in_cells: UVec2::ONE,
                },
            ),
        ])
    }

    fn cell_visual_layouts() -> CellVisualPlacementConfigMap {
        CellVisualPlacementConfigMap::from_entries(vec![
            (
                crate::plugins::default_plugin::boundary_cell_material(),
                VisualPlacementConfig {
                    label: "Boundary".to_string(),
                    draw_priority: 1000,
                    size_in_cells: UVec2::ONE,
                },
            ),
            (
                crate::plugins::default_plugin::brick_cell_material(),
                VisualPlacementConfig {
                    label: "Brick".to_string(),
                    draw_priority: 1000,
                    size_in_cells: UVec2::ONE,
                },
            ),
            (
                crate::plugins::default_plugin::metal_cell_material(),
                VisualPlacementConfig {
                    label: "Metal".to_string(),
                    draw_priority: 1000,
                    size_in_cells: UVec2::ONE,
                },
            ),
        ])
    }

    fn flatten_blocks(
        blocks: &[crate::ui::cell_inspector_model::CellInspectorBlockView],
    ) -> String {
        blocks
            .iter()
            .map(|block| {
                let mut text = block.title.clone();
                if !block.lines.is_empty() {
                    text.push('\n');
                    text.push_str(&block.lines.join("\n"));
                }
                text
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    #[test]
    fn pipe_and_vent_use_separate_blocks() {
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
        pipe_gas.add_species_counts(0, &[7, 2, 0]);

        let blocks = build_cell_inspector_blocks(
            UVec2::new(12, 14),
            &world,
            &gas,
            &pipe_config(),
            &registry,
            &world_hud_config(),
            &cell_visual_layouts(),
            &structure_hud_config(),
            &structure_visuals(),
            &structures,
            &pipe_gas,
            &flow_state,
            true,
        );

        let titles = blocks
            .iter()
            .map(|block| block.title.as_str())
            .collect::<Vec<_>>();
        assert!(titles.contains(&"Pipe"));
        assert!(titles.contains(&"Vent"));
        assert!(!titles.contains(&"Pipe + Vent"));
    }

    #[test]
    fn overlay_name_is_not_rendered_in_hud() {
        let registry = registry();
        let world = WorldGrid::default();
        let gas = GasField::from_registry(&registry);
        let structures = PlacedStructureMap::default();
        let pipe_gas = PipeGasField::from_registry(&registry);

        let blocks = build_cell_inspector_blocks(
            UVec2::new(4, 5),
            &world,
            &gas,
            &pipe_config(),
            &registry,
            &world_hud_config(),
            &cell_visual_layouts(),
            &structure_hud_config(),
            &structure_visuals(),
            &structures,
            &pipe_gas,
            &PipeFlowVisualState::default(),
            false,
        );
        let text = flatten_blocks(&blocks);

        assert!(!text.contains("F1"));
        assert!(!text.contains("F2"));
        assert!(!text.contains("F3"));
    }

    #[test]
    fn dominant_gas_format_includes_impurities() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut gas = GasField::from_registry(&registry);
        gas.set_amount(3, 4, 0, 1_901.0);
        gas.set_amount(3, 4, 1, 98.0);
        gas.set_amount(3, 4, 2, 1.0);

        let text = flatten_blocks(&build_cell_inspector_blocks(
            UVec2::new(3, 4),
            &world,
            &gas,
            &pipe_config(),
            &registry,
            &world_hud_config(),
            &cell_visual_layouts(),
            &structure_hud_config(),
            &structure_visuals(),
            &PlacedStructureMap::default(),
            &PipeGasField::from_registry(&registry),
            &PipeFlowVisualState::default(),
            false,
        ));

        assert!(
            text.contains("Gases: Hydrogen 95.1% (impurities: Oxygen 4.9%, Carbon dioxide < 0.1%)")
        );
    }

    #[test]
    fn mixed_gas_format_lists_all_parts_in_descending_order() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut gas = GasField::from_registry(&registry);
        gas.set_amount(8, 2, 0, 600.0);
        gas.set_amount(8, 2, 1, 300.0);
        gas.set_amount(8, 2, 2, 100.0);

        let text = flatten_blocks(&build_cell_inspector_blocks(
            UVec2::new(8, 2),
            &world,
            &gas,
            &pipe_config(),
            &registry,
            &world_hud_config(),
            &cell_visual_layouts(),
            &structure_hud_config(),
            &structure_visuals(),
            &PlacedStructureMap::default(),
            &PipeGasField::from_registry(&registry),
            &PipeFlowVisualState::default(),
            false,
        ));

        assert!(text.contains("Gases: Hydrogen 60%, Oxygen 30%, Carbon dioxide 10%"));
    }

    #[test]
    fn bridge_block_shows_gas_only_on_center_cell() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        assert!(structures
            .place_bridge(UVec2::new(20, 20), StructureRotation::Deg0, &world)
            .is_some());
        pipe_gas.sync_to_structures(&structures);
        pipe_gas.add_species_counts(0, &[10, 5, 0]);

        let edge_blocks = build_cell_inspector_blocks(
            UVec2::new(20, 20),
            &world,
            &GasField::from_registry(&registry),
            &pipe_config(),
            &registry,
            &world_hud_config(),
            &cell_visual_layouts(),
            &structure_hud_config(),
            &structure_visuals(),
            &structures,
            &pipe_gas,
            &PipeFlowVisualState::default(),
            false,
        );
        let edge_bridge = edge_blocks
            .iter()
            .find(|block| block.title == "Bridge")
            .expect("bridge block on edge");
        assert!(edge_bridge.lines.is_empty());

        let center_blocks = build_cell_inspector_blocks(
            UVec2::new(21, 20),
            &world,
            &GasField::from_registry(&registry),
            &pipe_config(),
            &registry,
            &world_hud_config(),
            &cell_visual_layouts(),
            &structure_hud_config(),
            &structure_visuals(),
            &structures,
            &pipe_gas,
            &PipeFlowVisualState::default(),
            false,
        );
        let center_bridge = center_blocks
            .iter()
            .find(|block| block.title == "Bridge")
            .expect("bridge block on center");
        assert!(center_bridge
            .lines
            .iter()
            .any(|line| line.starts_with("Pressure: ")));
        assert!(center_bridge
            .lines
            .iter()
            .any(|line| line == "Particles: 15"));
    }

    #[test]
    fn non_gas_entities_render_title_only_blocks() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        assert!(structures.place_vent(3, 3, &world));
        let blocks = build_cell_inspector_blocks(
            UVec2::new(3, 3),
            &world,
            &GasField::from_registry(&registry),
            &pipe_config(),
            &registry,
            &world_hud_config(),
            &cell_visual_layouts(),
            &structure_hud_config(),
            &structure_visuals(),
            &structures,
            &PipeGasField::from_registry(&registry),
            &PipeFlowVisualState::default(),
            false,
        );

        let vent = blocks
            .iter()
            .find(|block| block.title == "Vent")
            .expect("vent block");
        assert!(vent.lines.is_empty());
    }

    #[test]
    fn solid_materials_render_their_own_blocks() {
        let registry = registry();
        let mut world = WorldGrid::default();
        assert!(world.set_solid_with_material(
            6,
            6,
            crate::plugins::default_plugin::metal_cell_material()
        ));

        let blocks = build_cell_inspector_blocks(
            UVec2::new(6, 6),
            &world,
            &GasField::from_registry(&registry),
            &pipe_config(),
            &registry,
            &world_hud_config(),
            &cell_visual_layouts(),
            &structure_hud_config(),
            &structure_visuals(),
            &PlacedStructureMap::default(),
            &PipeGasField::from_registry(&registry),
            &PipeFlowVisualState::default(),
            false,
        );

        let metal = blocks
            .iter()
            .find(|block| block.title == "Metal")
            .expect("metal block");
        assert!(metal.lines.is_empty());
    }

    #[test]
    fn solid_cell_hud_has_material_block_without_cell_state_line() {
        let registry = registry();
        let mut world = WorldGrid::default();
        assert!(world.set_solid_with_material(
            7,
            7,
            crate::plugins::default_plugin::brick_cell_material()
        ));

        let blocks = build_cell_inspector_blocks(
            UVec2::new(7, 7),
            &world,
            &GasField::from_registry(&registry),
            &pipe_config(),
            &registry,
            &world_hud_config(),
            &cell_visual_layouts(),
            &structure_hud_config(),
            &structure_visuals(),
            &PlacedStructureMap::default(),
            &PipeGasField::from_registry(&registry),
            &PipeFlowVisualState::default(),
            false,
        );

        let cell_block = blocks
            .iter()
            .find(|block| block.title == "Cell")
            .expect("cell block");
        assert!(
            !cell_block
                .lines
                .iter()
                .any(|line| line.starts_with("State: ")),
            "cell block must not duplicate entity state line"
        );
        assert!(
            blocks.iter().any(|block| block.title == "Brick"),
            "solid cell must render a separate entity block with material name"
        );
    }

    #[test]
    fn hud_uses_registry_labels_without_hardcoded_gases() {
        let registry = GasRegistry::new(vec![
            GasDefinition {
                id: "ne".to_string(),
                label: "Neon".to_string(),
                color: [0.8, 0.4, 0.3],
                molecular_mass: 20.18,
            },
            GasDefinition {
                id: "ar".to_string(),
                label: "Argon".to_string(),
                color: [0.4, 0.8, 0.6],
                molecular_mass: 39.95,
            },
            GasDefinition {
                id: "kr".to_string(),
                label: "Krypton".to_string(),
                color: [0.3, 0.7, 0.9],
                molecular_mass: 83.80,
            },
            GasDefinition {
                id: "xe".to_string(),
                label: "Xenon".to_string(),
                color: [0.9, 0.8, 0.3],
                molecular_mass: 131.29,
            },
        ])
        .expect("custom registry");
        let world = WorldGrid::default();
        let mut gas = GasField::from_registry(&registry);
        gas.set_amount(7, 7, 0, 500.0);
        gas.set_amount(7, 7, 2, 500.0);

        let text = flatten_blocks(&build_cell_inspector_blocks(
            UVec2::new(7, 7),
            &world,
            &gas,
            &pipe_config(),
            &registry,
            &world_hud_config(),
            &cell_visual_layouts(),
            &structure_hud_config(),
            &structure_visuals(),
            &PlacedStructureMap::default(),
            &PipeGasField::from_registry(&registry),
            &PipeFlowVisualState::default(),
            false,
        ));

        assert!(text.contains("Neon 50%"));
        assert!(text.contains("Krypton 50%"));
        assert!(!text.contains("Hydrogen"));
        assert!(!text.contains("Oxygen"));
        assert!(!text.contains("Carbon dioxide"));
    }

    #[test]
    fn empty_container_keeps_pressure_and_particles_without_composition() {
        let registry = registry();
        let world = WorldGrid::default();
        let text = flatten_blocks(&build_cell_inspector_blocks(
            UVec2::new(1, 1),
            &world,
            &GasField::from_registry(&registry),
            &pipe_config(),
            &registry,
            &world_hud_config(),
            &cell_visual_layouts(),
            &structure_hud_config(),
            &structure_visuals(),
            &PlacedStructureMap::default(),
            &PipeGasField::from_registry(&registry),
            &PipeFlowVisualState::default(),
            false,
        ));

        assert!(text.contains("Pressure: 0Pa"));
        assert!(text.contains("Particles: 0"));
        assert!(!text.contains("Gases:"));
    }

    #[test]
    fn pipe_block_uses_transient_flow_when_requested() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        assert!(structures.place_pipe(8, 9, &world));
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        pipe_gas.sync_to_structures(&structures);
        let flow_state = PipeFlowVisualState {
            transfers: vec![PipeTransferRecord {
                from: UVec2::new(8, 9),
                from_kind: PipeContainerKind::Pipe,
                to: UVec2::new(9, 9),
                to_kind: PipeContainerKind::Pipe,
                gas_counts: vec![4, 1, 0],
                total_amount: 5,
                visual_path: PipeTransferVisualPath::Straight,
            }],
            ..Default::default()
        };

        let text = flatten_blocks(&build_cell_inspector_blocks(
            UVec2::new(8, 9),
            &world,
            &GasField::from_registry(&registry),
            &pipe_config(),
            &registry,
            &world_hud_config(),
            &cell_visual_layouts(),
            &structure_hud_config(),
            &structure_visuals(),
            &structures,
            &pipe_gas,
            &flow_state,
            true,
        ));

        assert!(text.contains("Particles: 5"));
        assert!(text.contains("Hydrogen 80%, Oxygen 20%"));
    }

    #[test]
    fn pipe_block_ignores_transient_flow_when_not_requested() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        assert!(structures.place_pipe(8, 9, &world));
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        pipe_gas.sync_to_structures(&structures);
        let flow_state = PipeFlowVisualState {
            transfers: vec![PipeTransferRecord {
                from: UVec2::new(8, 9),
                from_kind: PipeContainerKind::Pipe,
                to: UVec2::new(9, 9),
                to_kind: PipeContainerKind::Pipe,
                gas_counts: vec![4, 1, 0],
                total_amount: 5,
                visual_path: PipeTransferVisualPath::Straight,
            }],
            ..Default::default()
        };

        let text = flatten_blocks(&build_cell_inspector_blocks(
            UVec2::new(8, 9),
            &world,
            &GasField::from_registry(&registry),
            &pipe_config(),
            &registry,
            &world_hud_config(),
            &cell_visual_layouts(),
            &structure_hud_config(),
            &structure_visuals(),
            &structures,
            &pipe_gas,
            &flow_state,
            false,
        ));

        assert!(text.contains("Particles: 0"));
        assert!(!text.contains("Hydrogen 80%, Oxygen 20%"));
    }

    #[test]
    fn percentage_formatter_drops_trailing_zero() {
        assert_eq!(format_percentage_ratio(0.6), "60%");
        assert_eq!(format_percentage_ratio(0.049), "4.9%");
        assert_eq!(format_percentage_ratio(0.0005), "< 0.1%");
    }
}
