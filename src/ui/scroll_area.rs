use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    ui::{ComputedNode, RelativeCursorPosition, ScrollPosition},
    window::PrimaryWindow,
};

use crate::ui::palette;

const SCROLLBAR_THUMB_MIN_HEIGHT: f32 = 18.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
/// Defines which UI layer owns scroll interaction priority.
pub enum ScrollAreaInteractionGroup {
    #[default]
    Panel,
    Modal,
}

impl ScrollAreaInteractionGroup {
    fn sort_rank(self) -> i32 {
        match self {
            Self::Panel => 0,
            Self::Modal => 1,
        }
    }
}

#[derive(Component, Clone, Copy, Debug)]
/// Marks one generic scrollable viewport and stores its input priority settings.
pub struct ScrollAreaViewport {
    pub enabled: bool,
    pub interaction_group: ScrollAreaInteractionGroup,
    pub input_priority: i32,
}

impl ScrollAreaViewport {
    /// Builds a viewport configuration for panel UI.
    pub const fn panel(input_priority: i32) -> Self {
        Self {
            enabled: true,
            interaction_group: ScrollAreaInteractionGroup::Panel,
            input_priority,
        }
    }

    /// Builds a viewport configuration for modal UI.
    pub const fn modal(input_priority: i32) -> Self {
        Self {
            enabled: true,
            interaction_group: ScrollAreaInteractionGroup::Modal,
            input_priority,
        }
    }
}

impl Default for ScrollAreaViewport {
    fn default() -> Self {
        Self::panel(0)
    }
}

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

#[derive(Clone, Copy, Debug)]
/// Configures the placement of one generic scrollbar track.
pub struct ScrollAreaScrollbarStyle {
    pub right_px: f32,
    pub top_px: f32,
    pub bottom_px: f32,
    pub width_px: f32,
}

impl Default for ScrollAreaScrollbarStyle {
    fn default() -> Self {
        Self {
            right_px: 2.0,
            top_px: 0.0,
            bottom_px: 0.0,
            width_px: 6.0,
        }
    }
}

#[derive(Resource, Default, Clone, Copy, Debug)]
/// Blocks background panel scrolling while a modal UI captures wheel input.
pub struct UiScrollBlockState {
    pub block_panel_scrolling: bool,
}

#[derive(Resource, Default, Clone, Copy, Debug)]
struct ScrollAreaDragState {
    active: Option<ActiveScrollAreaDrag>,
}

#[derive(Clone, Copy, Debug)]
struct ActiveScrollAreaDrag {
    viewport: Entity,
    cursor_to_thumb_top_px: f32,
}

#[derive(Clone, Copy)]
struct ScrollAreaMetrics {
    viewport_height_px: f32,
    content_height_px: f32,
    max_scroll_logical: f32,
}

#[derive(Clone, Copy)]
struct ScrollAreaTargetCandidate {
    viewport: Entity,
    interaction_group: ScrollAreaInteractionGroup,
    input_priority: i32,
    global_z: i32,
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

fn scroll_offset_from_track_click(
    cursor_y_px: f32,
    thumb_height_px: f32,
    track_height_px: f32,
    max_scroll: f32,
) -> f32 {
    let travel = (track_height_px - thumb_height_px).max(0.0);
    if travel <= f32::EPSILON || max_scroll <= f32::EPSILON {
        return 0.0;
    }

    let thumb_top = (cursor_y_px - (thumb_height_px * 0.5)).clamp(0.0, travel);
    (thumb_top / travel) * max_scroll
}

fn scroll_offset_from_thumb_drag(
    cursor_y_px: f32,
    cursor_to_thumb_top_px: f32,
    thumb_height_px: f32,
    track_height_px: f32,
    max_scroll: f32,
) -> f32 {
    let travel = (track_height_px - thumb_height_px).max(0.0);
    if travel <= f32::EPSILON || max_scroll <= f32::EPSILON {
        return 0.0;
    }

    let thumb_top = (cursor_y_px - cursor_to_thumb_top_px).clamp(0.0, travel);
    (thumb_top / travel) * max_scroll
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

fn viewport_is_scroll_target_allowed(
    viewport: &ScrollAreaViewport,
    scroll_block: UiScrollBlockState,
) -> bool {
    viewport.enabled
        && !(scroll_block.block_panel_scrolling
            && viewport.interaction_group == ScrollAreaInteractionGroup::Panel)
}

fn choose_topmost_scroll_target(
    candidates: impl IntoIterator<Item = ScrollAreaTargetCandidate>,
) -> Option<Entity> {
    candidates
        .into_iter()
        .max_by_key(|candidate| {
            (
                candidate.interaction_group.sort_rank(),
                candidate.input_priority,
                candidate.global_z,
            )
        })
        .map(|candidate| candidate.viewport)
}

fn apply_scroll_area_scrolling(
    mut mouse_wheel: EventReader<MouseWheel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    scroll_block: Res<UiScrollBlockState>,
    mut viewport_nodes: Query<
        (
            Entity,
            &ComputedNode,
            &mut ScrollPosition,
            &RelativeCursorPosition,
            &Node,
            Option<&GlobalZIndex>,
            &ScrollAreaViewport,
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

    let Some(target_viewport) = choose_topmost_scroll_target(viewport_nodes.iter_mut().filter_map(
        |(entity, computed, _, relative_cursor, node, global_z, viewport)| {
            if node.display == Display::None
                || !normalized_cursor_is_inside(relative_cursor.normalized)
                || scroll_area_metrics(computed).max_scroll_logical <= 1.0
                || !viewport_is_scroll_target_allowed(viewport, *scroll_block)
            {
                return None;
            }

            Some(ScrollAreaTargetCandidate {
                viewport: entity,
                interaction_group: viewport.interaction_group,
                input_priority: viewport.input_priority,
                global_z: global_z.map(|value| value.0).unwrap_or(0),
            })
        },
    )) else {
        return;
    };

    for (entity, computed, mut scroll_position, _, _, _, _) in &mut viewport_nodes {
        if entity != target_viewport {
            continue;
        }
        let metrics = scroll_area_metrics(computed);
        scroll_position.offset_y =
            next_scroll_offset(scroll_position.offset_y, delta, metrics.max_scroll_logical);
    }
}

fn handle_scroll_area_mouse_input(
    buttons: Res<ButtonInput<MouseButton>>,
    scroll_block: Res<UiScrollBlockState>,
    mut drag_state: ResMut<ScrollAreaDragState>,
    mut viewport_queries: ParamSet<(
        Query<
            (
                Entity,
                &ComputedNode,
                &ScrollPosition,
                &ScrollAreaViewport,
                &Node,
                Option<&GlobalZIndex>,
            ),
            With<ScrollAreaViewport>,
        >,
        Query<&mut ScrollPosition, With<ScrollAreaViewport>>,
    )>,
    track_nodes: Query<
        (
            &ScrollAreaScrollbarTrack,
            &ComputedNode,
            &RelativeCursorPosition,
            &Node,
        ),
        With<ScrollAreaScrollbarTrack>,
    >,
) {
    if buttons.just_released(MouseButton::Left) || !buttons.pressed(MouseButton::Left) {
        drag_state.active = None;
    }

    if let Some(active_drag) = drag_state.active {
        let (viewport_entity, metrics, enabled) = {
            let viewport_query = viewport_queries.p0();
            let Ok((viewport_entity, computed, _, viewport, node, _)) =
                viewport_query.get(active_drag.viewport)
            else {
                drag_state.active = None;
                return;
            };
            (
                viewport_entity,
                scroll_area_metrics(computed),
                node.display != Display::None
                    && viewport_is_scroll_target_allowed(viewport, *scroll_block),
            )
        };
        if !enabled {
            drag_state.active = None;
            return;
        }
        let mut scroll_positions = viewport_queries.p1();
        let Ok(mut scroll_position) = scroll_positions.get_mut(viewport_entity) else {
            drag_state.active = None;
            return;
        };
        if metrics.max_scroll_logical <= 1.0 {
            drag_state.active = None;
            return;
        }

        for (track, track_computed, relative_cursor, track_node) in &track_nodes {
            if track.viewport != viewport_entity || track_node.display == Display::None {
                continue;
            }
            let Some(cursor) = relative_cursor.normalized else {
                continue;
            };
            let track_height = track_computed.size().y.max(0.0);
            let Some((_, thumb_height)) = scrollbar_thumb_layout(
                metrics.viewport_height_px,
                metrics.content_height_px,
                track_height,
                scroll_position.offset_y,
                metrics.max_scroll_logical,
            ) else {
                drag_state.active = None;
                return;
            };
            let cursor_y_px = cursor.y * track_height;
            scroll_position.offset_y = scroll_offset_from_thumb_drag(
                cursor_y_px,
                active_drag.cursor_to_thumb_top_px,
                thumb_height,
                track_height,
                metrics.max_scroll_logical,
            );
            break;
        }
        return;
    }

    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let mut best_target: Option<(ScrollAreaTargetCandidate, f32, f32, f32, f32, f32)> = None;
    for (track, track_computed, relative_cursor, track_node) in &track_nodes {
        if track_node.display == Display::None
            || !normalized_cursor_is_inside(relative_cursor.normalized)
        {
            continue;
        }
        let (
            viewport_entity,
            offset_y,
            metrics,
            interaction_group,
            input_priority,
            global_z,
            enabled,
        ) = {
            let viewport_query = viewport_queries.p0();
            let Ok((viewport_entity, computed, scroll_position, viewport, node, global_z)) =
                viewport_query.get(track.viewport)
            else {
                continue;
            };
            (
                viewport_entity,
                scroll_position.offset_y,
                scroll_area_metrics(computed),
                viewport.interaction_group,
                viewport.input_priority,
                global_z.map(|value| value.0).unwrap_or(0),
                node.display != Display::None
                    && viewport_is_scroll_target_allowed(viewport, *scroll_block),
            )
        };
        if !enabled || metrics.max_scroll_logical <= 1.0 {
            continue;
        }
        let track_height = track_computed.size().y.max(0.0);
        let Some((thumb_top, thumb_height)) = scrollbar_thumb_layout(
            metrics.viewport_height_px,
            metrics.content_height_px,
            track_height,
            offset_y,
            metrics.max_scroll_logical,
        ) else {
            continue;
        };
        let cursor_y_px = relative_cursor.normalized.unwrap_or_default().y * track_height;
        let candidate = (
            ScrollAreaTargetCandidate {
                viewport: viewport_entity,
                interaction_group,
                input_priority,
                global_z,
            },
            cursor_y_px,
            thumb_top,
            thumb_height,
            track_height,
            metrics.max_scroll_logical,
        );
        let candidate_key = (
            candidate.0.interaction_group.sort_rank(),
            candidate.0.input_priority,
            candidate.0.global_z,
        );
        let is_better = best_target
            .as_ref()
            .map(|(existing, ..)| {
                let existing_key = (
                    existing.interaction_group.sort_rank(),
                    existing.input_priority,
                    existing.global_z,
                );
                candidate_key > existing_key
            })
            .unwrap_or(true);
        if is_better {
            best_target = Some(candidate);
        }
    }

    let Some((candidate, cursor_y_px, thumb_top, thumb_height, track_height, max_scroll)) =
        best_target
    else {
        return;
    };

    let mut scroll_positions = viewport_queries.p1();
    let Ok(mut scroll_position) = scroll_positions.get_mut(candidate.viewport) else {
        return;
    };
    if (thumb_top..=thumb_top + thumb_height).contains(&cursor_y_px) {
        drag_state.active = Some(ActiveScrollAreaDrag {
            viewport: candidate.viewport,
            cursor_to_thumb_top_px: cursor_y_px - thumb_top,
        });
        return;
    }

    scroll_position.offset_y =
        scroll_offset_from_track_click(cursor_y_px, thumb_height, track_height, max_scroll);
}

fn sync_scroll_area_scrollbar_visuals(
    mut queries: ParamSet<(
        Query<
            (
                Entity,
                &ComputedNode,
                &ScrollPosition,
                &ScrollAreaViewport,
                &Node,
            ),
            With<ScrollAreaViewport>,
        >,
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
    for (entity, computed, scroll_position, viewport, node) in &queries.p0() {
        viewport_data.insert(
            entity,
            (
                scroll_area_metrics(computed),
                scroll_position.offset_y,
                viewport.enabled && node.display != Display::None,
            ),
        );
    }

    let mut track_height_by_viewport = std::collections::HashMap::new();
    for (track, computed) in &queries.p1() {
        track_height_by_viewport.insert(track.viewport, computed.size().y.max(0.0));
    }

    for (maybe_track, maybe_thumb, mut node, mut visibility) in &mut queries.p2() {
        if let Some(track) = maybe_track {
            let Some((metrics, _, enabled)) = viewport_data.get(&track.viewport).copied() else {
                *visibility = Visibility::Hidden;
                node.display = Display::None;
                continue;
            };
            let show = enabled && metrics.max_scroll_logical > 1.0;
            *visibility = if show {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
            node.display = if show { Display::Flex } else { Display::None };
            continue;
        }

        if let Some(thumb) = maybe_thumb {
            let Some((metrics, offset_y, enabled)) = viewport_data.get(&thumb.viewport).copied()
            else {
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
            *visibility = if enabled {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}

/// Spawns one scrollbar track + thumb pair for a generic scroll area viewport.
pub fn spawn_scroll_area_scrollbar(parent: &mut ChildSpawnerCommands, viewport: Entity) {
    spawn_scroll_area_scrollbar_with_style(parent, viewport, ScrollAreaScrollbarStyle::default());
}

/// Spawns one scrollbar track + thumb pair with explicit placement config.
pub fn spawn_scroll_area_scrollbar_with_style(
    parent: &mut ChildSpawnerCommands,
    viewport: Entity,
    style: ScrollAreaScrollbarStyle,
) {
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(style.right_px),
                top: Val::Px(style.top_px),
                bottom: Val::Px(style.bottom_px),
                width: Val::Px(style.width_px),
                display: Display::None,
                ..default()
            },
            BackgroundColor(palette::PANEL_SCROLLBAR_TRACK_BG),
            Visibility::Hidden,
            RelativeCursorPosition::default(),
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
        app.init_resource::<UiScrollBlockState>()
            .init_resource::<ScrollAreaDragState>()
            .add_systems(
                Update,
                (
                    apply_scroll_area_scrolling,
                    handle_scroll_area_mouse_input,
                    sync_scroll_area_scrollbar_visuals,
                ),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        choose_topmost_scroll_target, next_scroll_offset, normalized_cursor_is_inside,
        scroll_offset_from_thumb_drag, scroll_offset_from_track_click, scrollbar_thumb_layout,
        ScrollAreaInteractionGroup, ScrollAreaTargetCandidate,
    };
    use bevy::prelude::{Entity, Vec2};

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

    #[test]
    fn track_click_centers_thumb_before_mapping_to_scroll() {
        let offset = scroll_offset_from_track_click(90.0, 40.0, 180.0, 400.0);
        assert!((offset - 200.0).abs() < 0.001);
    }

    #[test]
    fn thumb_drag_uses_grab_offset() {
        let offset = scroll_offset_from_thumb_drag(120.0, 10.0, 40.0, 180.0, 400.0);
        assert!((offset - 314.2857).abs() < 0.001);
    }

    #[test]
    fn target_selection_prefers_modal_then_priority_then_z() {
        let selected = choose_topmost_scroll_target([
            ScrollAreaTargetCandidate {
                viewport: Entity::from_raw(1),
                interaction_group: ScrollAreaInteractionGroup::Panel,
                input_priority: 10,
                global_z: 100,
            },
            ScrollAreaTargetCandidate {
                viewport: Entity::from_raw(2),
                interaction_group: ScrollAreaInteractionGroup::Modal,
                input_priority: 0,
                global_z: 1,
            },
            ScrollAreaTargetCandidate {
                viewport: Entity::from_raw(3),
                interaction_group: ScrollAreaInteractionGroup::Modal,
                input_priority: 1,
                global_z: 0,
            },
        ]);
        assert_eq!(selected, Some(Entity::from_raw(3)));
    }
}
