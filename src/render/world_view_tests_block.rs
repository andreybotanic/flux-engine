#[cfg(test)]
mod tests {
    use super::{
        bridge_arc_control_point, bridge_bend_direction, bridge_packet_position,
        bridge_visual_size, bridge_visual_transform,
        build_pipe_mask_image, build_vent_overlay_image, build_world_fade_mask_image, grid_fade,
        bridge_curve_progress_for_transfer,
        cursor_highlight_segments,
        flow_packet_position, pipe_flow_packet_visible, pipe_gas_square_size,
        pipe_overlay_block_offset, pipe_overlay_block_visible,
        quadratic_bezier_point, straight_packet_position,
        pipe_highlight_visibility, vent_world_visibility, world_fade_alpha,
        overlay_gas_visibility, structure_visual_key, wall_sprite_path_for_material,
        OverlayMode,
    };
    use crate::config::{
        CellVisualPlacementConfigMap, GasDefinition, GasRegistry, StructureVisualConfigMap,
        VisualPlacementConfig,
    };
    use crate::plugins::default_plugin::pipe_runtime::{
        pipe_cell_display_total_particles_with_transfers, PipeContainerKind, PipeFlowVisualState,
        PipeGasField, PipeSimulationConfig, PipeTransferRecord, PipeTransferVisualPath,
    };
    use bevy::prelude::Visibility;
    use bevy::math::{UVec2, Vec2, Vec3};
    use std::collections::HashSet;
    use crate::world::{
        grid::{WorldGrid, CELL_SIZE},
        structures::{PlacedStructureMap, StructureRotation},
    };

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

    #[test]
    fn grid_fade_is_full_at_center_and_zero_beyond_radius() {
        assert!((grid_fade(0.0, 8.0) - 1.0).abs() < 1e-6);
        assert_eq!(grid_fade(8.0, 8.0), 0.0);
        assert_eq!(grid_fade(9.5, 8.0), 0.0);
    }

    #[test]
    fn grid_fade_decreases_smoothly_with_distance() {
        let near = grid_fade(1.0, 8.0);
        let mid = grid_fade(4.0, 8.0);
        let far = grid_fade(7.0, 8.0);
        assert!(near > mid && mid > far && far > 0.0);
    }

    #[test]
    fn world_fade_alpha_is_zero_inside_and_one_at_outer_edge() {
        assert_eq!(world_fade_alpha(0.0, 64.0), 0.0);
        assert!(world_fade_alpha(32.0, 64.0) > 0.0);
        assert!((world_fade_alpha(64.0, 64.0) - 1.0).abs() < 1e-6);
        assert!((world_fade_alpha(100.0, 64.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn generated_world_fade_mask_has_expected_alpha_profile() {
        let world_size = Vec2::new(160.0, 96.0);
        let image = build_world_fade_mask_image(world_size);
        let data = image.data.as_ref().expect("generated image has pixel data");
        let width = image.texture_descriptor.size.width as usize;
        let height = image.texture_descriptor.size.height as usize;
        let center_x = width / 2;
        let center_y = height / 2;
        let center_idx = ((center_y * width + center_x) * 4) + 3;
        let corner_idx = 3;
        let edge_mid_idx = (((height / 2) * width) * 4) + 3;
        let center_alpha = data[center_idx];
        let edge_alpha = data[edge_mid_idx];
        let corner_alpha = data[corner_idx];
        assert_eq!(center_alpha, 0);
        assert!(edge_alpha > 0);
        assert_eq!(corner_alpha, 255);
    }

    #[test]
    fn pipe_mask_vertical_connections_match_world_orientation() {
        let up_visual = build_pipe_mask_image(0b0001);
        let down_visual = build_pipe_mask_image(0b0100);
        let up_data = up_visual
            .data
            .as_ref()
            .expect("pipe mask image has pixel data");
        let down_data = down_visual
            .data
            .as_ref()
            .expect("pipe mask image has pixel data");
        let width = up_visual.texture_descriptor.size.width as usize;
        let sample_alpha = |data: &[u8], x: usize, y: usize| data[((y * width + x) * 4) + 3];

        assert!(sample_alpha(up_data, width / 2, 8) > 0);
        assert_eq!(sample_alpha(up_data, width / 2, width - 8), 0);
        assert_eq!(sample_alpha(down_data, width / 2, 8), 0);
        assert!(sample_alpha(down_data, width / 2, width - 8) > 0);
    }

    #[test]
    fn pipe_mask_connections_reach_cell_edges() {
        let right_visual = build_pipe_mask_image(0b0010);
        let left_visual = build_pipe_mask_image(0b1000);
        let up_visual = build_pipe_mask_image(0b0001);
        let down_visual = build_pipe_mask_image(0b0100);
        let right_data = right_visual
            .data
            .as_ref()
            .expect("pipe mask image has pixel data");
        let left_data = left_visual
            .data
            .as_ref()
            .expect("pipe mask image has pixel data");
        let down_data = down_visual
            .data
            .as_ref()
            .expect("pipe mask image has pixel data");
        let up_data = up_visual
            .data
            .as_ref()
            .expect("pipe mask image has pixel data");
        let width = right_visual.texture_descriptor.size.width as usize;
        let sample_alpha = |data: &[u8], x: usize, y: usize| data[((y * width + x) * 4) + 3];
        let c = width / 2;

        assert!(sample_alpha(right_data, width - 1, c) > 0);
        assert!(sample_alpha(left_data, 0, c) > 0);
        assert!(sample_alpha(down_data, c, width - 1) > 0);
        assert!(sample_alpha(up_data, c, 0) > 0);
    }

    #[test]
    fn pipe_mask_without_connections_stays_compact_in_center() {
        let visual = build_pipe_mask_image(0);
        let data = visual
            .data
            .as_ref()
            .expect("pipe mask image has pixel data");
        let width = visual.texture_descriptor.size.width as usize;
        let sample_alpha = |x: usize, y: usize| data[((y * width + x) * 4) + 3];
        let c = width / 2;

        assert!(sample_alpha(c, c) > 0);
        assert_eq!(sample_alpha(8, c), 0);
        assert_eq!(sample_alpha(width - 8, c), 0);
        assert_eq!(sample_alpha(c, 8), 0);
        assert_eq!(sample_alpha(c, width - 8), 0);
    }

    #[test]
    fn default_visual_priorities_keep_pipe_below_bridge_and_walls() {
        let structure_visuals = structure_visuals();
        let cell_visual_layouts = cell_visual_layouts();
        assert!(structure_visuals.get(crate::plugins::default_plugin::pipe_structure_kind()).draw_priority
            < structure_visuals
                .get(crate::plugins::default_plugin::gas_pipe_bridge_structure_kind())
                .draw_priority);
        assert!(structure_visuals.get(crate::plugins::default_plugin::gas_pipe_bridge_structure_kind()).draw_priority
            < cell_visual_layouts.get(crate::plugins::default_plugin::brick_cell_material()).draw_priority);
    }

    #[test]
    fn wall_sprite_path_uses_registered_descriptor_for_metal_material() {
        let registry = crate::plugins::default_plugin::default_content_registry();
        let path = wall_sprite_path_for_material(
            &registry,
            crate::plugins::default_plugin::metal_cell_material(),
        );
        assert_eq!(path, "flux_default://world/tile_metal.ktx2");
    }

    #[test]
    fn structure_visual_keys_do_not_collide_for_pipe_and_vent_in_same_cell() {
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        assert!(structures.place_pipe(12, 12, &world));
        assert!(structures.place_vent(12, 12, &world));

        let keys = structures
            .iter()
            .map(structure_visual_key)
            .collect::<HashSet<_>>();

        assert_eq!(keys.len(), structures.iter().count());
    }

    #[test]
    fn wall_sprite_path_uses_registered_descriptor_for_brick_material() {
        let registry = crate::plugins::default_plugin::default_content_registry();
        let path = wall_sprite_path_for_material(
            &registry,
            crate::plugins::default_plugin::brick_cell_material(),
        );
        assert_eq!(path, "flux_default://world/tile_brick.ktx2");
    }

    #[test]
    fn pipe_highlight_filter_is_visible_only_in_f3() {
        assert_eq!(
            pipe_highlight_visibility(true, OverlayMode::Main, true),
            Visibility::Hidden
        );
        assert_eq!(
            pipe_highlight_visibility(true, OverlayMode::Gas, true),
            Visibility::Hidden
        );
        assert_eq!(
            pipe_highlight_visibility(
                true,
                crate::plugins::default_plugin::pipes_overlay_mode(),
                true,
            ),
            Visibility::Visible
        );
        assert_eq!(
            pipe_highlight_visibility(
                false,
                crate::plugins::default_plugin::pipes_overlay_mode(),
                true,
            ),
            Visibility::Hidden
        );
    }

    #[test]
    fn vent_overlay_is_square_with_transparent_interior_and_arrows() {
        let image = build_vent_overlay_image();
        let data = image
            .data
            .as_ref()
            .expect("vent overlay image has pixel data");
        let width = image.texture_descriptor.size.width as usize;
        let sample_alpha = |x: usize, y: usize| data[((y * width + x) * 4) + 3];

        assert_eq!(sample_alpha(4, 4), 0);
        assert!(sample_alpha(14, 20) > 0);
        assert!(sample_alpha(32, 8) > 0);
        assert!(sample_alpha(32, 46) > 0);
        assert_eq!(sample_alpha(20, 32), 0);
    }

    #[test]
    fn world_vent_sprite_stays_visible_in_f3() {
        assert_eq!(
            vent_world_visibility(true, OverlayMode::Main),
            Visibility::Visible
        );
        assert_eq!(
            vent_world_visibility(true, OverlayMode::Gas),
            Visibility::Visible
        );
        assert_eq!(
            vent_world_visibility(true, crate::plugins::default_plugin::pipes_overlay_mode()),
            Visibility::Visible
        );
        assert_eq!(
            vent_world_visibility(false, crate::plugins::default_plugin::pipes_overlay_mode()),
            Visibility::Hidden
        );
    }

    #[test]
    fn plugin_graph_overlay_hides_core_gas_main_sprite() {
        let (_, gas_main) = overlay_gas_visibility(OverlayMode::plugin("flux.default.overlay.pipes"), true);
        assert_eq!(gas_main, Visibility::Hidden);
    }

    #[test]
    fn pipes_overlay_hides_core_gas_main_sprite_even_without_graph() {
        let (_, gas_main) = overlay_gas_visibility(OverlayMode::plugin("flux.default.overlay.pipes"), false);
        assert_eq!(gas_main, Visibility::Hidden);
    }

    #[test]
    fn pipe_gas_square_size_scales_with_amount_and_hits_expected_max() {
        let config = pipe_config();
        let low = pipe_gas_square_size(&config, 1);
        let mid = pipe_gas_square_size(&config, 500);
        let full = pipe_gas_square_size(&config, 1_000);
        let denser = pipe_gas_square_size(&config, 10_000);

        assert!(low > 0.0);
        assert!(mid > low);
        assert!(full > mid);
        assert!(denser > full);
        assert!(denser < super::CELL_SIZE * 0.63);
    }

    #[test]
    fn empty_pipe_overlay_blocks_are_hidden() {
        assert!(!pipe_overlay_block_visible(0));
        assert!(pipe_overlay_block_visible(1));
    }

    #[test]
    fn non_zero_pipe_flow_packets_are_visible() {
        assert!(!pipe_flow_packet_visible(0));
        assert!(pipe_flow_packet_visible(1));
        assert!(pipe_flow_packet_visible(4));
        assert!(pipe_flow_packet_visible(5));
        assert!(pipe_flow_packet_visible(12));
    }

    #[test]
    fn pipe_display_total_uses_flow_packets_when_storage_is_empty() {
        let registry = registry();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        assert!(structures.place_pipe(3, 4, &world));
        assert!(structures.place_pipe(4, 4, &world));
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        pipe_gas.sync_to_structures(&structures);
        let flow_state = PipeFlowVisualState {
            transfers: vec![PipeTransferRecord {
                from: UVec2::new(3, 4),
                from_kind: PipeContainerKind::Pipe,
                to: UVec2::new(4, 4),
                to_kind: PipeContainerKind::Pipe,
                gas_counts: vec![6, 2],
                total_amount: 8,
                visual_path: PipeTransferVisualPath::Straight,
            }],
            ..Default::default()
        };

        assert_eq!(
            pipe_cell_display_total_particles_with_transfers(
                &structures,
                &pipe_gas,
                &flow_state,
                3,
                4,
                true,
            ),
            8
        );
        assert_eq!(
            pipe_cell_display_total_particles_with_transfers(
                &structures,
                &pipe_gas,
                &flow_state,
                4,
                4,
                true,
            ),
            8
        );
    }

    #[test]
    fn bridge_visual_keeps_base_horizontal_size_for_all_rotations() {
        let structure_visuals = structure_visuals();
        assert_eq!(
            bridge_visual_size(StructureRotation::Deg0, &structure_visuals),
            Vec2::new(CELL_SIZE * 3.0, CELL_SIZE)
        );
        assert_eq!(
            bridge_visual_size(StructureRotation::Deg90, &structure_visuals),
            Vec2::new(CELL_SIZE * 3.0, CELL_SIZE)
        );
    }

    #[test]
    fn bridge_vertical_variant_is_produced_by_rotation_not_resizing() {
        let transform = bridge_visual_transform(UVec2::new(10, 11), StructureRotation::Deg90, 0.25);
        let rotated = transform.rotation * Vec3::X;
        assert!(rotated.y > 0.99);
    }

    #[test]
    fn quadratic_bezier_preserves_start_and_end_points() {
        let start = Vec2::new(10.0, 12.0);
        let control = Vec2::new(18.0, 24.0);
        let end = Vec2::new(40.0, 8.0);

        assert_eq!(quadratic_bezier_point(start, control, end, 0.0), start);
        assert_eq!(quadratic_bezier_point(start, control, end, 1.0), end);
    }

    #[test]
    fn horizontal_bridge_packet_arc_bends_upward() {
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        assert!(structures
            .place_bridge(UVec2::new(10, 10), StructureRotation::Deg0, &world)
            .is_some());
        let transfer = PipeTransferRecord {
            from: UVec2::new(11, 10),
            from_kind: PipeContainerKind::BridgePipe,
            to: UVec2::new(10, 10),
            to_kind: PipeContainerKind::BridgePipe,
            gas_counts: vec![5, 0],
            total_amount: 5,
            visual_path: PipeTransferVisualPath::BridgeArc {
                bridge_origin: UVec2::new(10, 10),
                bridge_rotation: StructureRotation::Deg0,
            },
        };
        let midpoint = bridge_packet_position(&transfer, 0.5)
            .expect("bridge transfer should use arc");
        let straight_midpoint = straight_packet_position(
            crate::world::grid::cell_center(11, 10),
            crate::world::grid::cell_center(10, 10),
            0.5,
        );

        assert!(midpoint.y > straight_midpoint.y);
    }

    #[test]
    fn bridge_arc_control_point_uses_stronger_offset() {
        let center = UVec2::new(10, 10);
        let center_world = crate::world::grid::cell_center(center.x, center.y);
        let control = bridge_arc_control_point(center, StructureRotation::Deg0);

        assert!((control.y - center_world.y - CELL_SIZE * 0.45).abs() < 1e-4);
    }

    #[test]
    fn vertical_bridge_packet_arc_bends_left() {
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        assert!(structures
            .place_bridge(UVec2::new(10, 10), StructureRotation::Deg90, &world)
            .is_some());
        let transfer = PipeTransferRecord {
            from: UVec2::new(10, 11),
            from_kind: PipeContainerKind::BridgePipe,
            to: UVec2::new(10, 10),
            to_kind: PipeContainerKind::BridgePipe,
            gas_counts: vec![5, 0],
            total_amount: 5,
            visual_path: PipeTransferVisualPath::BridgeArc {
                bridge_origin: UVec2::new(10, 10),
                bridge_rotation: StructureRotation::Deg90,
            },
        };
        let midpoint = bridge_packet_position(&transfer, 0.5)
            .expect("bridge transfer should use arc");
        let straight_midpoint = straight_packet_position(
            crate::world::grid::cell_center(10, 11),
            crate::world::grid::cell_center(10, 10),
            0.5,
        );

        assert!(midpoint.x < straight_midpoint.x);
    }

    #[test]
    fn bridge_overlay_offset_follows_bridge_bend_direction() {
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        assert!(structures
            .place_bridge(UVec2::new(10, 10), StructureRotation::Deg90, &world)
            .is_some());

        let bridge_offset = pipe_overlay_block_offset(
            0,
            2,
            PipeContainerKind::BridgePipe,
            &structures,
            UVec2::new(10, 11),
        );
        let plain_offset = pipe_overlay_block_offset(
            1,
            2,
            PipeContainerKind::Pipe,
            &structures,
            UVec2::new(10, 11),
        );

        assert!(bridge_offset.x < 0.0);
        assert!((plain_offset + bridge_offset).length_squared() < 1e-6);
    }

    #[test]
    fn bridge_packet_halves_share_one_common_apex() {
        let center = UVec2::new(11, 10);
        let first_port = UVec2::new(10, 10);
        let second_port = UVec2::new(12, 10);

        let left_to_center = PipeTransferRecord {
            from: first_port,
            from_kind: PipeContainerKind::BridgePipe,
            to: center,
            to_kind: PipeContainerKind::BridgePipe,
            gas_counts: vec![5, 0],
            total_amount: 5,
            visual_path: PipeTransferVisualPath::BridgeArc {
                bridge_origin: UVec2::new(10, 10),
                bridge_rotation: StructureRotation::Deg0,
            },
        };
        let center_to_right = PipeTransferRecord {
            from: center,
            from_kind: PipeContainerKind::BridgePipe,
            to: second_port,
            to_kind: PipeContainerKind::BridgePipe,
            gas_counts: vec![5, 0],
            total_amount: 5,
            visual_path: PipeTransferVisualPath::BridgeArc {
                bridge_origin: UVec2::new(10, 10),
                bridge_rotation: StructureRotation::Deg0,
            },
        };

        let left_end = bridge_curve_progress_for_transfer(
            &left_to_center,
            center,
            first_port,
            second_port,
            1.0,
        )
        .expect("left half");
        let right_start = bridge_curve_progress_for_transfer(
            &center_to_right,
            center,
            first_port,
            second_port,
            0.0,
        )
        .expect("right half");

        assert!((left_end - 0.5).abs() < 1e-6);
        assert!((right_start - 0.5).abs() < 1e-6);
    }

    #[test]
    fn future_bridge_rotations_have_expected_arc_directions() {
        let center = UVec2::new(20, 21);
        let origin = crate::world::grid::cell_center(center.x, center.y);

        assert_eq!(bridge_bend_direction(StructureRotation::Deg0), Vec2::Y);
        assert_eq!(bridge_bend_direction(StructureRotation::Deg90), Vec2::NEG_X);
        assert!(bridge_arc_control_point(center, StructureRotation::Deg180).y < origin.y);
        assert!(bridge_arc_control_point(center, StructureRotation::Deg270).x > origin.x);
    }

    #[test]
    fn non_bridge_transfers_keep_straight_packet_motion() {
        let transfer = PipeTransferRecord {
            from: UVec2::new(3, 4),
            from_kind: PipeContainerKind::Pipe,
            to: UVec2::new(4, 4),
            to_kind: PipeContainerKind::Pipe,
            gas_counts: vec![6, 2],
            total_amount: 8,
            visual_path: PipeTransferVisualPath::Straight,
        };
        let position = flow_packet_position(&transfer, 0.5);
        let expected = straight_packet_position(
            crate::world::grid::cell_center(3, 4),
            crate::world::grid::cell_center(4, 4),
            0.5,
        );

        assert_eq!(position, expected);
    }

    #[test]
    fn bridge_arc_detection_matches_only_center_to_port_pairs() {
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        assert!(structures
            .place_bridge(UVec2::new(10, 10), StructureRotation::Deg0, &world)
            .is_some());
        let non_bridge_transfer = PipeTransferRecord {
            from: UVec2::new(11, 10),
            from_kind: PipeContainerKind::Pipe,
            to: UVec2::new(11, 11),
            to_kind: PipeContainerKind::Pipe,
            gas_counts: vec![3, 0],
            total_amount: 3,
            visual_path: PipeTransferVisualPath::Straight,
        };

        assert!(bridge_packet_position(&non_bridge_transfer, 0.5).is_none());
    }

    #[test]
    fn plain_pipe_under_bridge_keeps_straight_packet_motion() {
        let transfer = PipeTransferRecord {
            from: UVec2::new(11, 10),
            from_kind: PipeContainerKind::Pipe,
            to: UVec2::new(10, 10),
            to_kind: PipeContainerKind::Pipe,
            gas_counts: vec![5, 0],
            total_amount: 5,
            visual_path: PipeTransferVisualPath::Straight,
        };

        let position = flow_packet_position(&transfer, 0.5);
        let expected = straight_packet_position(
            crate::world::grid::cell_center(11, 10),
            crate::world::grid::cell_center(10, 10),
            0.5,
        );

        assert_eq!(position, expected);
    }

    #[test]
    fn cursor_highlight_segments_stay_inside_hovered_cell() {
        let cell = UVec2::new(10, 10);
        let center = crate::world::grid::cell_center(cell.x, cell.y);
        let min_x = center.x - CELL_SIZE * 0.5;
        let max_x = center.x + CELL_SIZE * 0.5;
        let min_y = center.y - CELL_SIZE * 0.5;
        let max_y = center.y + CELL_SIZE * 0.5;

        for (start, end) in cursor_highlight_segments(cell) {
            for point in [start, end] {
                assert!(point.x >= min_x && point.x <= max_x);
                assert!(point.y >= min_y && point.y <= max_y);
            }
        }
    }

    #[test]
    fn cursor_highlight_segments_stay_close_to_cell_border() {
        let cell = UVec2::new(10, 10);
        let center = crate::world::grid::cell_center(cell.x, cell.y);
        let min_x = center.x - CELL_SIZE * 0.5;
        let max_x = center.x + CELL_SIZE * 0.5;
        let min_y = center.y - CELL_SIZE * 0.5;
        let max_y = center.y + CELL_SIZE * 0.5;

        for (start, end) in cursor_highlight_segments(cell) {
            if (start.y - end.y).abs() < 1e-6 {
                let border_distance = (max_y - start.y).abs().min((start.y - min_y).abs());
                assert!(border_distance <= 1.0);
            }
            if (start.x - end.x).abs() < 1e-6 {
                let border_distance = (max_x - start.x).abs().min((start.x - min_x).abs());
                assert!(border_distance <= 1.0);
            }
        }
    }

    #[test]
    fn cursor_highlight_segments_are_symmetric_between_opposite_sides() {
        let cell = UVec2::new(12, 7);
        let center = crate::world::grid::cell_center(cell.x, cell.y);
        let segments = cursor_highlight_segments(cell);
        let top = segments
            .iter()
            .filter(|(start, end)| start.y == end.y && start.y > center.y)
            .collect::<Vec<_>>();
        let bottom = segments
            .iter()
            .filter(|(start, end)| start.y == end.y && start.y < center.y)
            .collect::<Vec<_>>();
        let left = segments
            .iter()
            .filter(|(start, end)| start.x == end.x && start.x < center.x)
            .collect::<Vec<_>>();
        let right = segments
            .iter()
            .filter(|(start, end)| start.x == end.x && start.x > center.x)
            .collect::<Vec<_>>();

        assert_eq!(top.len(), bottom.len());
        assert_eq!(left.len(), right.len());
        for (top_segment, bottom_segment) in top.iter().zip(bottom.iter()) {
            assert!((top_segment.0.x - bottom_segment.0.x).abs() < 1e-6);
            assert!((top_segment.1.x - bottom_segment.1.x).abs() < 1e-6);
        }
        for (left_segment, right_segment) in left.iter().zip(right.iter()) {
            assert!((left_segment.0.y - right_segment.0.y).abs() < 1e-6);
            assert!((left_segment.1.y - right_segment.1.y).abs() < 1e-6);
        }
    }

    #[test]
    fn cursor_highlight_segments_use_expected_dash_count() {
        assert_eq!(cursor_highlight_segments(UVec2::new(0, 0)).len(), 16);
    }
}
