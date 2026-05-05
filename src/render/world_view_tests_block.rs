#[cfg(test)]
mod tests {
    use super::grid_fade;

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
}
