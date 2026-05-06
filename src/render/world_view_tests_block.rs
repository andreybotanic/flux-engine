#[cfg(test)]
mod tests {
    use super::{
        bridge_visual_size, bridge_visual_transform,
        build_pipe_mask_image, build_vent_overlay_image, build_world_fade_mask_image, grid_fade,
        pipe_flow_packet_visual, pipe_flow_square_size, pipe_gas_square_size,
        pipe_highlight_visibility, pipe_world_z, vent_world_visibility, world_fade_alpha,
        OverlayMode,
    };
    use crate::simulation::pipes::{
        pipe_cell_display_total_particles_with_transfers, PipeFlowVisualState, PipeGasField,
        PipeTransferRecord,
    };
    use bevy::prelude::Visibility;
    use bevy::math::{UVec2, Vec2, Vec3};
    use crate::config::{GasDefinition, GasRegistry};
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
    fn pipe_layer_is_below_walls_in_world_and_above_them_in_f3() {
        assert!(pipe_world_z(OverlayMode::Main) < 0.5);
        assert!(pipe_world_z(OverlayMode::Gas) < 0.5);
        assert!(pipe_world_z(OverlayMode::Pipes) > 0.5);
    }

    #[test]
    fn pipe_highlight_filter_is_visible_only_in_f3() {
        assert_eq!(
            pipe_highlight_visibility(true, OverlayMode::Main),
            Visibility::Hidden
        );
        assert_eq!(
            pipe_highlight_visibility(true, OverlayMode::Gas),
            Visibility::Hidden
        );
        assert_eq!(
            pipe_highlight_visibility(true, OverlayMode::Pipes),
            Visibility::Visible
        );
        assert_eq!(
            pipe_highlight_visibility(false, OverlayMode::Pipes),
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
            vent_world_visibility(true, OverlayMode::Pipes),
            Visibility::Visible
        );
        assert_eq!(
            vent_world_visibility(false, OverlayMode::Pipes),
            Visibility::Hidden
        );
    }

    #[test]
    fn pipe_gas_square_size_scales_with_amount_and_hits_expected_max() {
        let low = pipe_gas_square_size(1);
        let mid = pipe_gas_square_size(500);
        let full = pipe_gas_square_size(1_000);

        assert!(low > 0.0);
        assert!(mid > low);
        assert!(full > mid);
        assert!((full - super::CELL_SIZE * 0.70).abs() < 1e-6);
    }

    #[test]
    fn pipe_flow_square_size_stays_smaller_than_static_square() {
        let static_half = pipe_gas_square_size(500);
        let flow_half = pipe_flow_square_size(500);
        let flow_full = pipe_flow_square_size(1_000);

        assert!(flow_half > 0.0);
        assert!(flow_full > flow_half);
        assert!(flow_half < static_half);
        assert!((flow_full - super::CELL_SIZE * 0.42).abs() < 1e-6);
    }

    #[test]
    fn pipe_flow_packet_visual_gets_brighter_and_less_transparent_with_more_gas() {
        let low = pipe_flow_packet_visual(1);
        let mid = pipe_flow_packet_visual(500);
        let full = pipe_flow_packet_visual(1_000);

        assert!(low.intensity < mid.intensity && mid.intensity < full.intensity);
        assert!(low.fill_alpha < mid.fill_alpha && mid.fill_alpha < full.fill_alpha);
        assert!(low.fill_alpha < 0.2);
        assert!((full.fill_alpha - 0.92).abs() < 1e-6);
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
                to: UVec2::new(4, 4),
                gas_counts: vec![6, 2],
                total_amount: 8,
            }],
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
        assert_eq!(
            bridge_visual_size(StructureRotation::Deg0),
            Vec2::new(CELL_SIZE * 3.0, CELL_SIZE)
        );
        assert_eq!(
            bridge_visual_size(StructureRotation::Deg90),
            Vec2::new(CELL_SIZE * 3.0, CELL_SIZE)
        );
    }

    #[test]
    fn bridge_vertical_variant_is_produced_by_rotation_not_resizing() {
        let transform =
            bridge_visual_transform(UVec2::new(10, 11), StructureRotation::Deg90, OverlayMode::Main);
        let rotated = transform.rotation * Vec3::X;
        assert!(rotated.y > 0.99);
    }
}
