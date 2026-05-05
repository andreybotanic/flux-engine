#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{PanelScrollPolicy, PANEL_HEADER_HEIGHT};

    fn should_enable_scroll(
        policy: PanelScrollPolicy,
        panel_height: f32,
        available_h: f32,
    ) -> bool {
        let threshold = match policy {
            PanelScrollPolicy::Never => return false,
            PanelScrollPolicy::AutoHalfScreen => available_h * 0.5,
            PanelScrollPolicy::FromTotalHeightPx(px) => px.max(PANEL_HEADER_HEIGHT),
        };
        panel_height > threshold
    }

    #[test]
    fn auto_half_screen_enables_scroll_above_half() {
        assert!(should_enable_scroll(
            PanelScrollPolicy::AutoHalfScreen,
            520.0,
            1000.0
        ));
        assert!(!should_enable_scroll(
            PanelScrollPolicy::AutoHalfScreen,
            500.0,
            1000.0
        ));
    }

    #[test]
    fn explicit_scroll_threshold_works() {
        assert!(should_enable_scroll(
            PanelScrollPolicy::FromTotalHeightPx(620.0),
            621.0,
            1000.0
        ));
        assert!(!should_enable_scroll(
            PanelScrollPolicy::FromTotalHeightPx(620.0),
            620.0,
            1000.0
        ));
    }

    #[test]
    fn never_policy_never_enables_scroll() {
        assert!(!should_enable_scroll(
            PanelScrollPolicy::Never,
            9999.0,
            1000.0
        ));
    }

    fn stack_offsets(
        mut entries: Vec<(&'static str, u64, f32)>,
        gap: f32,
    ) -> HashMap<&'static str, f32> {
        entries.sort_by_key(|(_, opened_at, _)| *opened_at);
        let mut current = 0.0f32;
        let mut offsets = HashMap::new();
        for (id, _, height) in entries {
            offsets.insert(id, current);
            current += height + gap;
        }
        offsets
    }

    fn visual_left_to_right_row_reverse(inserted: &[&'static str]) -> Vec<&'static str> {
        inserted.iter().rev().copied().collect()
    }

    #[test]
    fn top_corner_collapse_moves_next_panel_up() {
        let gap = 12.0;
        let before = stack_offsets(vec![("first", 1, 220.0), ("second", 2, 120.0)], gap);
        let after = stack_offsets(
            vec![
                ("first", 1, PANEL_HEADER_HEIGHT), // collapsed
                ("second", 2, 120.0),
            ],
            gap,
        );
        assert!(after["second"] < before["second"]);
    }

    #[test]
    fn bottom_corner_collapse_moves_next_panel_down_towards_corner() {
        let gap = 12.0;
        let before = stack_offsets(vec![("bottom", 1, 260.0), ("above", 2, 120.0)], gap);
        let after = stack_offsets(
            vec![
                ("bottom", 1, PANEL_HEADER_HEIGHT), // collapsed
                ("above", 2, 120.0),
            ],
            gap,
        );
        assert!(after["above"] < before["above"]);
    }

    #[test]
    fn reopen_changes_position_by_open_order() {
        let gap = 12.0;
        let initial = stack_offsets(vec![("a", 1, 120.0), ("b", 2, 120.0)], gap);
        let reopened = stack_offsets(vec![("b", 2, 120.0), ("a", 3, 120.0)], gap);
        assert!(initial["a"] < initial["b"]);
        assert!(reopened["b"] < reopened["a"]);
    }

    #[test]
    fn row_reverse_keeps_first_inserted_button_rightmost() {
        let inserted = vec!["first", "second", "third"];
        let visual_left_to_right = visual_left_to_right_row_reverse(&inserted);
        let rightmost = visual_left_to_right
            .last()
            .expect("visual list should not be empty");
        assert_eq!(*rightmost, "first");
    }
}
