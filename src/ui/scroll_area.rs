use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    ui::{ComputedNode, RelativeCursorPosition, ScrollPosition},
    window::PrimaryWindow,
};

use crate::ui::palette;

const SCROLLBAR_THUMB_MIN_HEIGHT: f32 = 18.0;

#[derive(Component, Clone, Copy)]
/// Marks a generic scrollable UI viewport that reacts to mouse wheel input.
pub struct ScrollAreaViewport;

#[derive(Component, Clone, Copy)]
/// Stores the scrollbar track linked to one `ScrollAreaViewport`.
pub struct ScrollAreaScrollbarTrack {
    pub viewport: Entity,
}

#[derive(Component, Clone, Copy)]
/// Stores the scrollbar thumb linked to one `ScrollAreaViewport`.
pub struct ScrollAreaScrollbarThumb {
    pub viewport: Entity,
}

#[derive(Resource, Default, Clone, Copy, Debug)]
/// Blocks background panel scrolling while a modal UI captures wheel input.
pub struct UiScrollBlockState {
    pub block_panel_scrolling: bool,
}

#[derive(Clone, Copy)]
struct ScrollAreaMetrics {
    viewport_height_px: f32,
    content_height_px: f32,
    max_scroll_logical: f32,
}

fn scroll_area_metrics(computed: &ComputedNode) -> ScrollAreaMetrics {
    let viewport_height_px = computed.size().y.max(0.0);
    let content_height_px = computed.content_size().y.max(viewport_height_px);
    let max_scroll_logical = ((content_height_px - viewport_height_px).max(0.0)
        * computed.inverse_scale_factor())
    .max(0.0);

    ScrollAreaMetrics {
        viewport_height_px,
        content_height_px,
        max_scroll_logical,
    }
}

fn normalized_cursor_is_inside(normalized: Option<Vec2>) -> bool {
    let Some(cursor) = normalized else {
        return false;
    };
    (0.0..=1.0).contains(&cursor.x) && (0.0..=1.0).contains(&cursor.y)
}

fn next_scroll_offset(current: f32, delta: f32, max_scroll: f32) -> f32 {
    (current - delta).clamp(0.0, max_scroll.max(0.0))
}

fn scrollbar_thumb_layout(
    viewport_height_px: f32,
    content_height_px: f32,
    track_height_px: f32,
    offset_y: f32,
    max_scroll: f32,
) -> Option<(f32, f32)> {
    if max_scroll <= 1.0 || track_height_px <= 1.0 || content_height_px <= 0.0 {
        return None;
    }

    let ratio = (viewport_height_px / content_height_px).clamp(0.0, 1.0);
    let thumb_height = (ratio * track_height_px).clamp(
        SCROLLBAR_THUMB_MIN_HEIGHT,
        track_height_px.max(SCROLLBAR_THUMB_MIN_HEIGHT),
    );
    let travel = (track_height_px - thumb_height).max(0.0);
    let scroll_ratio = (offset_y / max_scroll).clamp(0.0, 1.0);
    Some((scroll_ratio * travel, thumb_height))
}

fn apply_scroll_area_scrolling(
    mut mouse_wheel: EventReader<MouseWheel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut viewport_nodes: Query<
        (
            Entity,
            &ComputedNode,
            &mut ScrollPosition,
            &RelativeCursorPosition,
            &Node,
            Option<&GlobalZIndex>,
        ),
        With<ScrollAreaViewport>,
    >,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    if window.cursor_position().is_none() {
        return;
    }

    let mut target_viewport = None;
    let mut best_z = i32::MIN;
    for (entity, computed, _, relative_cursor, node, global_z) in &mut viewport_nodes {
        if node.display == Display::None
            || !normalized_cursor_is_inside(relative_cursor.normalized)
            || scroll_area_metrics(computed).max_scroll_logical <= 1.0
        {
            continue;
        }
        let z = global_z.map(|value| value.0).unwrap_or(0);
        if z >= best_z {
            best_z = z;
            target_viewport = Some(entity);
        }
    }

    let Some(target_viewport) = target_viewport else {
        return;
    };

    let mut delta = 0.0f32;
    for event in mouse_wheel.read() {
        let y = match event.unit {
            MouseScrollUnit::Line => event.y * 36.0,
            MouseScrollUnit::Pixel => event.y,
        };
        delta += y;
    }
    if delta.abs() <= f32::EPSILON {
        return;
    }

    for (entity, computed, mut scroll_position, _, _, _) in &mut viewport_nodes {
        if entity != target_viewport {
            continue;
        }
        let metrics = scroll_area_metrics(computed);
        scroll_position.offset_y =
            next_scroll_offset(scroll_position.offset_y, delta, metrics.max_scroll_logical);
    }
}

fn sync_scroll_area_scrollbar_visuals(
    viewport_nodes: Query<(Entity, &ComputedNode, &ScrollPosition), With<ScrollAreaViewport>>,
    mut scrollbar_queries: ParamSet<(
        Query<(&ScrollAreaScrollbarTrack, &ComputedNode)>,
        Query<
            (
                Option<&ScrollAreaScrollbarTrack>,
                Option<&ScrollAreaScrollbarThumb>,
                &mut Node,
                &mut Visibility,
            ),
            Or<(
                With<ScrollAreaScrollbarTrack>,
                With<ScrollAreaScrollbarThumb>,
            )>,
        >,
    )>,
) {
    let mut viewport_data = std::collections::HashMap::new();
    for (entity, computed, scroll_position) in &viewport_nodes {
        viewport_data.insert(
            entity,
            (scroll_area_metrics(computed), scroll_position.offset_y),
        );
    }

    let mut track_height_by_viewport = std::collections::HashMap::new();
    for (track, computed) in &scrollbar_queries.p0() {
        track_height_by_viewport.insert(track.viewport, computed.size().y.max(0.0));
    }

    for (maybe_track, maybe_thumb, mut node, mut visibility) in &mut scrollbar_queries.p1() {
        if let Some(track) = maybe_track {
            let Some((metrics, _)) = viewport_data.get(&track.viewport).copied() else {
                *visibility = Visibility::Hidden;
                node.display = Display::None;
                continue;
            };
            let show = metrics.max_scroll_logical > 1.0;
            *visibility = if show {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
            node.display = if show { Display::Flex } else { Display::None };
            continue;
        }

        if let Some(thumb) = maybe_thumb {
            let Some((metrics, offset_y)) = viewport_data.get(&thumb.viewport).copied() else {
                *visibility = Visibility::Hidden;
                continue;
            };
            let track_height = track_height_by_viewport
                .get(&thumb.viewport)
                .copied()
                .unwrap_or(0.0);
            let Some((thumb_top, thumb_height)) = scrollbar_thumb_layout(
                metrics.viewport_height_px,
                metrics.content_height_px,
                track_height,
                offset_y,
                metrics.max_scroll_logical,
            ) else {
                *visibility = Visibility::Hidden;
                continue;
            };

            node.top = Val::Px(thumb_top);
            node.height = Val::Px(thumb_height);
            *visibility = Visibility::Visible;
        }
    }
}

/// Spawns one scrollbar track + thumb pair for a generic scroll area viewport.
pub fn spawn_scroll_area_scrollbar(parent: &mut ChildSpawnerCommands, viewport: Entity) {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(2.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                width: Val::Px(6.0),
                display: Display::None,
                ..default()
            },
            BackgroundColor(palette::PANEL_SCROLLBAR_TRACK_BG),
            Visibility::Hidden,
            ScrollAreaScrollbarTrack { viewport },
        ))
        .with_children(|track| {
            track.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    height: Val::Px(SCROLLBAR_THUMB_MIN_HEIGHT),
                    ..default()
                },
                BackgroundColor(palette::PANEL_SCROLLBAR_THUMB_BG),
                Visibility::Hidden,
                ScrollAreaScrollbarThumb { viewport },
            ));
        });
}

/// Stores `ScrollAreaPlugin` state.
pub struct ScrollAreaPlugin;

impl Plugin for ScrollAreaPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiScrollBlockState>().add_systems(
            Update,
            (
                apply_scroll_area_scrolling,
                sync_scroll_area_scrollbar_visuals,
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{next_scroll_offset, normalized_cursor_is_inside, scrollbar_thumb_layout};
    use bevy::prelude::Vec2;

    #[test]
    fn normalized_cursor_check_accepts_points_inside_unit_rect() {
        assert!(normalized_cursor_is_inside(Some(Vec2::new(0.0, 0.0))));
        assert!(normalized_cursor_is_inside(Some(Vec2::new(0.5, 0.5))));
        assert!(normalized_cursor_is_inside(Some(Vec2::new(1.0, 1.0))));
        assert!(!normalized_cursor_is_inside(Some(Vec2::new(-0.1, 0.5))));
        assert!(!normalized_cursor_is_inside(Some(Vec2::new(0.5, 1.1))));
        assert!(!normalized_cursor_is_inside(None));
    }

    #[test]
    fn next_scroll_offset_clamps_to_bounds() {
        assert_eq!(next_scroll_offset(20.0, 50.0, 100.0), 0.0);
        assert_eq!(next_scroll_offset(20.0, -200.0, 100.0), 100.0);
        assert_eq!(next_scroll_offset(40.0, 10.0, 100.0), 30.0);
    }

    #[test]
    fn scrollbar_thumb_layout_returns_none_without_overflow() {
        assert_eq!(scrollbar_thumb_layout(200.0, 200.0, 200.0, 0.0, 0.0), None);
    }

    #[test]
    fn scrollbar_thumb_layout_scales_and_moves_thumb() {
        let (top, height) =
            scrollbar_thumb_layout(200.0, 600.0, 180.0, 50.0, 400.0).expect("thumb layout");
        assert!(height >= 18.0);
        assert!(top > 0.0);
        assert!(top + height <= 180.0 + f32::EPSILON);
    }
}
