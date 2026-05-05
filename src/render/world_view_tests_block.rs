#[cfg(test)]
mod tests {
    use super::{build_world_fade_mask_image, grid_fade, world_fade_alpha};
    use bevy::math::Vec2;

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
}
