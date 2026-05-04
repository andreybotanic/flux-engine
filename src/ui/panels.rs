use std::collections::HashMap;

use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    ui::{ComputedNode, ScrollPosition},
    window::PrimaryWindow,
};

const PANEL_HEADER_HEIGHT: f32 = 34.0;
const PANEL_CONTENT_PADDING_Y: f32 = 10.0;
const PANEL_HEADER_BUTTON_SIZE: f32 = 24.0;
const PANEL_HEADER_BUTTON_BG: Color = Color::srgba(0.70, 0.73, 0.77, 0.96);
const PANEL_HEADER_TEXT: Color = Color::srgba(0.10, 0.10, 0.12, 1.0);
const SCROLLBAR_TRACK_WIDTH: f32 = 6.0;
const SCROLLBAR_THUMB_MIN_HEIGHT: f32 = 18.0;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PanelId(&'static str);

impl PanelId {
    pub const fn new(id: &'static str) -> Self {
        Self(id)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PanelHeaderActionId(&'static str);

impl PanelHeaderActionId {
    pub const fn new(id: &'static str) -> Self {
        Self(id)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PanelCorner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Clone, Debug)]
pub struct PanelControls {
    pub show_collapse: bool,
    pub show_close: bool,
    pub custom_actions: Vec<PanelHeaderActionId>,
}

impl Default for PanelControls {
    fn default() -> Self {
        Self {
            show_collapse: false,
            show_close: false,
            custom_actions: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum PanelScrollPolicy {
    AutoHalfScreen,
    Never,
    FromTotalHeightPx(f32),
}

#[derive(Clone, Debug)]
pub struct PanelSpec {
    pub id: PanelId,
    pub title: String,
    pub corner: PanelCorner,
    pub width: f32,
    pub margin_x: f32,
    pub margin_y: f32,
    pub stack_gap: f32,
    pub controls: PanelControls,
    pub scroll_policy: PanelScrollPolicy,
    pub background: Color,
    pub header_background: Color,
    pub initial_visible: bool,
    pub initial_collapsed: bool,
}

#[derive(Clone, Debug)]
pub struct PanelState {
    pub visible: bool,
    pub collapsed: bool,
    pub opened_at: u64,
    pub scroll_enabled: bool,
}

#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct PanelOpenOrder {
    next: u64,
}

impl PanelOpenOrder {
    pub fn allocate(&mut self) -> u64 {
        self.next = self.next.saturating_add(1);
        self.next
    }
}

#[derive(Clone, Copy, Debug)]
struct PanelRect {
    left: f32,
    top: f32,
    width: f32,
    height: f32,
}

impl PanelRect {
    fn contains(self, point: Vec2) -> bool {
        point.x >= self.left
            && point.x <= self.left + self.width
            && point.y >= self.top
            && point.y <= self.top + self.height
    }
}

#[derive(Clone, Copy, Debug)]
struct PanelHitRect {
    panel_id: PanelId,
    rect: PanelRect,
    global_z: i32,
    scrollable: bool,
}

#[derive(Clone, Copy, Debug)]
struct PanelPlacement {
    offset: f32,
}

impl PanelPlacement {
    fn new() -> Self {
        Self { offset: 0.0 }
    }
}

struct ManagedPanel {
    spec: PanelSpec,
    state: PanelState,
    root: Entity,
    content_inner: Entity,
    close_button: Option<Entity>,
    natural_total_height: f32,
    layout_height: f32,
}

#[derive(Resource, Default)]
pub struct PanelManager {
    panels: HashMap<PanelId, ManagedPanel>,
    hit_rects: Vec<PanelHitRect>,
}

impl PanelManager {
    pub fn spawn_panel(
        &mut self,
        commands: &mut Commands,
        open_order: &mut PanelOpenOrder,
        spec: PanelSpec,
        build_content: impl FnOnce(&mut ChildSpawnerCommands),
    ) {
        let initial_opened_at = if spec.initial_visible {
            open_order.allocate()
        } else {
            0
        };
        let state = PanelState {
            visible: spec.initial_visible,
            collapsed: spec.initial_collapsed,
            opened_at: initial_opened_at,
            scroll_enabled: false,
        };

        let panel_id = spec.id;
        let mut panel_root = commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(spec.width),
                display: if state.visible {
                    Display::Flex
                } else {
                    Display::None
                },
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(spec.background),
            GlobalZIndex(0),
            PanelRoot { panel_id },
        ));

        let mut close_button_entity = None;
        let mut content_inner_entity = None;

        panel_root.with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(PANEL_HEADER_HEIGHT),
                    min_height: Val::Px(PANEL_HEADER_HEIGHT),
                    max_height: Val::Px(PANEL_HEADER_HEIGHT),
                    padding: UiRect::axes(Val::Px(9.0), Val::Px(5.0)),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(spec.header_background),
            ))
            .with_children(|header| {
                header.spawn((
                    Text::new(spec.title.clone()),
                    TextFont::from_font_size(14.0),
                    TextColor(PANEL_HEADER_TEXT),
                ));

                header
                    .spawn(Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::RowReverse,
                        column_gap: Val::Px(6.0),
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|controls| {
                        if spec.controls.show_close {
                            let entity = controls
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(PANEL_HEADER_BUTTON_SIZE),
                                        height: Val::Px(PANEL_HEADER_BUTTON_SIZE),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(PANEL_HEADER_BUTTON_BG),
                                    PanelHeaderButton {
                                        panel_id,
                                        action: PanelHeaderAction::Close,
                                    },
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new("X"),
                                        TextFont::from_font_size(12.0),
                                        TextColor(PANEL_HEADER_TEXT),
                                    ));
                                })
                                .id();
                            close_button_entity = Some(entity);
                        }

                        if spec.controls.show_collapse {
                            let collapse_text = if state.collapsed { "+" } else { "-" };
                            let entity = controls
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(PANEL_HEADER_BUTTON_SIZE),
                                        height: Val::Px(PANEL_HEADER_BUTTON_SIZE),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(PANEL_HEADER_BUTTON_BG),
                                    PanelHeaderButton {
                                        panel_id,
                                        action: PanelHeaderAction::ToggleCollapse,
                                    },
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new(collapse_text),
                                        TextFont::from_font_size(14.0),
                                        TextColor(PANEL_HEADER_TEXT),
                                        PanelCollapseButtonLabel { panel_id },
                                    ));
                                })
                                .id();
                            let _ = entity;
                        }

                        for action in spec.controls.custom_actions.iter().copied() {
                            let entity = controls
                                .spawn((
                                    Button,
                                    Node {
                                        min_width: Val::Px(24.0),
                                        height: Val::Px(PANEL_HEADER_BUTTON_SIZE),
                                        padding: UiRect::axes(Val::Px(6.0), Val::Px(0.0)),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(PANEL_HEADER_BUTTON_BG),
                                    PanelHeaderButton {
                                        panel_id,
                                        action: PanelHeaderAction::Custom(action),
                                    },
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new(action.as_str()),
                                        TextFont::from_font_size(12.0),
                                        TextColor(PANEL_HEADER_TEXT),
                                    ));
                                })
                                .id();
                            let _ = (action, entity);
                        }
                    });
            });

            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    display: if state.collapsed {
                        Display::None
                    } else {
                        Display::Flex
                    },
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::clip_y(),
                    ..default()
                },
                ScrollPosition::default(),
                PanelContentViewport { panel_id },
            ))
            .with_children(|viewport| {
                let inner = viewport
                    .spawn((
                        Node {
                            width: Val::Percent(100.0),
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(8.0),
                            ..default()
                        },
                        PanelContentInner { panel_id },
                    ))
                    .with_children(build_content)
                    .id();
                content_inner_entity = Some(inner);
            });

            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(2.0),
                    top: Val::Px(PANEL_HEADER_HEIGHT + PANEL_CONTENT_PADDING_Y),
                    bottom: Val::Px(PANEL_CONTENT_PADDING_Y),
                    width: Val::Px(SCROLLBAR_TRACK_WIDTH),
                    display: Display::None,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.86, 0.88, 0.90, 0.82)),
                Visibility::Hidden,
                PanelScrollbarTrack { panel_id },
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
                    BackgroundColor(Color::srgba(0.18, 0.19, 0.22, 0.28)),
                    Visibility::Hidden,
                    PanelScrollbarThumb { panel_id },
                ));
            });
        });

        let root_entity = panel_root.id();
        self.panels.insert(
            panel_id,
            ManagedPanel {
                spec,
                state,
                root: root_entity,
                content_inner: content_inner_entity.expect("panel content inner must exist"),
                close_button: close_button_entity,
                natural_total_height: PANEL_HEADER_HEIGHT,
                layout_height: PANEL_HEADER_HEIGHT,
            },
        );
    }

    pub fn set_visible(
        &mut self,
        panel_id: PanelId,
        visible: bool,
        open_order: &mut PanelOpenOrder,
    ) -> bool {
        let Some(panel) = self.panels.get_mut(&panel_id) else {
            return false;
        };
        if panel.state.visible == visible {
            return false;
        }
        panel.state.visible = visible;
        if visible {
            panel.state.opened_at = open_order.allocate();
        }
        true
    }

    pub fn is_cursor_over_any_panel(&self, cursor: Vec2) -> bool {
        self.hit_rects.iter().any(|item| item.rect.contains(cursor))
    }

    pub fn topmost_scrollable_panel_at(&self, cursor: Vec2) -> Option<PanelId> {
        self.hit_rects
            .iter()
            .filter(|item| item.scrollable && item.rect.contains(cursor))
            .max_by_key(|item| item.global_z)
            .map(|item| item.panel_id)
    }
}

#[derive(Component, Clone, Copy)]
struct PanelRoot {
    panel_id: PanelId,
}

#[derive(Component, Clone, Copy)]
struct PanelContentViewport {
    panel_id: PanelId,
}

#[derive(Component, Clone, Copy)]
struct PanelContentInner {
    panel_id: PanelId,
}

#[derive(Component, Clone, Copy)]
struct PanelScrollbarTrack {
    panel_id: PanelId,
}

#[derive(Component, Clone, Copy)]
struct PanelScrollbarThumb {
    panel_id: PanelId,
}

#[derive(Component, Clone, Copy)]
struct PanelCollapseButtonLabel {
    panel_id: PanelId,
}

#[derive(Clone, Copy)]
enum PanelHeaderAction {
    ToggleCollapse,
    Close,
    Custom(PanelHeaderActionId),
}

#[derive(Component, Clone, Copy)]
struct PanelHeaderButton {
    panel_id: PanelId,
    action: PanelHeaderAction,
}

#[derive(Event, Clone, Copy, Debug)]
pub struct PanelHeaderActionEvent {
    pub panel_id: PanelId,
    pub action_id: PanelHeaderActionId,
}

pub struct PanelPlugin;

impl Plugin for PanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PanelManager>()
            .init_resource::<PanelOpenOrder>()
            .add_event::<PanelHeaderActionEvent>()
            .add_systems(
                Update,
                (
                    handle_panel_header_buttons,
                    sync_panel_measured_sizes,
                    apply_panel_layout,
                    sync_panel_visual_state,
                    apply_panel_scrolling,
                    sync_panel_scrollbar_visuals,
                    update_panel_hit_rects,
                )
                    .chain(),
            );
    }
}

fn handle_panel_header_buttons(
    mut interactions: Query<
        (&Interaction, &PanelHeaderButton),
        (Changed<Interaction>, With<Button>),
    >,
    mut panels: ResMut<PanelManager>,
    mut action_events: EventWriter<PanelHeaderActionEvent>,
) {
    for (interaction, button) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let Some(panel) = panels.panels.get_mut(&button.panel_id) else {
            continue;
        };
        match button.action {
            PanelHeaderAction::ToggleCollapse => {
                panel.state.collapsed = !panel.state.collapsed;
            }
            PanelHeaderAction::Close => {
                panel.state.visible = false;
            }
            PanelHeaderAction::Custom(action_id) => {
                action_events.write(PanelHeaderActionEvent {
                    panel_id: button.panel_id,
                    action_id,
                });
            }
        }
    }
}

fn sync_panel_measured_sizes(
    mut panels: ResMut<PanelManager>,
    content_nodes: Query<(&PanelContentInner, &ComputedNode)>,
) {
    for panel in panels.panels.values_mut() {
        if let Ok((_, computed)) = content_nodes.get(panel.content_inner) {
            panel.natural_total_height =
                (PANEL_HEADER_HEIGHT + (PANEL_CONTENT_PADDING_Y * 2.0) + computed.size().y)
                    .max(PANEL_HEADER_HEIGHT);
        }
    }
}

fn apply_panel_layout(
    mut panels: ResMut<PanelManager>,
    mut root_nodes: Query<(&mut Node, &mut GlobalZIndex), With<PanelRoot>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    let mut corner_top_left = PanelPlacement::new();
    let mut corner_top_right = PanelPlacement::new();
    let mut corner_bottom_left = PanelPlacement::new();
    let mut corner_bottom_right = PanelPlacement::new();

    let mut ids = panels
        .panels
        .iter()
        .filter_map(|(id, panel)| panel.state.visible.then_some((*id, panel.state.opened_at)))
        .collect::<Vec<_>>();
    ids.sort_by_key(|(_, opened_at)| *opened_at);

    for (panel_id, _) in ids {
        let Some(panel) = panels.panels.get_mut(&panel_id) else {
            continue;
        };
        let Ok((mut node, mut z)) = root_nodes.get_mut(panel.root) else {
            continue;
        };

        node.display = Display::Flex;
        node.width = Val::Px(panel.spec.width);
        node.left = Val::Auto;
        node.right = Val::Auto;
        node.top = Val::Auto;
        node.bottom = Val::Auto;
        *z = GlobalZIndex(panel.state.opened_at as i32);

        let natural_total = panel.natural_total_height.max(PANEL_HEADER_HEIGHT);
        let available_height = (window.height() - (panel.spec.margin_y * 2.0)).max(0.0);
        let threshold = match panel.spec.scroll_policy {
            PanelScrollPolicy::Never => None,
            PanelScrollPolicy::AutoHalfScreen => Some(available_height * 0.5),
            PanelScrollPolicy::FromTotalHeightPx(px) => Some(px.max(PANEL_HEADER_HEIGHT)),
        };
        let (scroll_enabled, panel_height) = if panel.state.collapsed {
            (false, PANEL_HEADER_HEIGHT)
        } else if let Some(max_total) = threshold {
            if natural_total > max_total {
                (true, max_total.max(PANEL_HEADER_HEIGHT))
            } else {
                (false, natural_total)
            }
        } else {
            (false, natural_total)
        };
        panel.state.scroll_enabled = scroll_enabled;
        panel.layout_height = panel_height;

        if panel.state.collapsed {
            node.height = Val::Px(PANEL_HEADER_HEIGHT);
            node.min_height = Val::Px(PANEL_HEADER_HEIGHT);
            node.max_height = Val::Px(PANEL_HEADER_HEIGHT);
        } else if panel.state.scroll_enabled {
            node.height = Val::Px(panel.layout_height);
            node.min_height = Val::Px(panel.layout_height);
            node.max_height = Val::Px(panel.layout_height);
        } else {
            node.height = Val::Auto;
            node.min_height = Val::Auto;
            node.max_height = Val::Auto;
        }

        match panel.spec.corner {
            PanelCorner::TopLeft => {
                node.left = Val::Px(panel.spec.margin_x);
                node.top = Val::Px(panel.spec.margin_y + corner_top_left.offset);
                corner_top_left.offset += panel_height + panel.spec.stack_gap;
            }
            PanelCorner::TopRight => {
                node.right = Val::Px(panel.spec.margin_x);
                node.top = Val::Px(panel.spec.margin_y + corner_top_right.offset);
                corner_top_right.offset += panel_height + panel.spec.stack_gap;
            }
            PanelCorner::BottomLeft => {
                node.left = Val::Px(panel.spec.margin_x);
                node.bottom = Val::Px(panel.spec.margin_y + corner_bottom_left.offset);
                corner_bottom_left.offset += panel_height + panel.spec.stack_gap;
            }
            PanelCorner::BottomRight => {
                node.right = Val::Px(panel.spec.margin_x);
                node.bottom = Val::Px(panel.spec.margin_y + corner_bottom_right.offset);
                corner_bottom_right.offset += panel_height + panel.spec.stack_gap;
            }
        }
    }

    for panel in panels.panels.values() {
        if panel.state.visible {
            continue;
        }
        if let Ok((mut node, _)) = root_nodes.get_mut(panel.root) {
            node.display = Display::None;
        }
    }
}

fn sync_panel_visual_state(
    panels: Res<PanelManager>,
    mut viewport_nodes: Query<(&PanelContentViewport, &mut Node, &mut ScrollPosition)>,
    mut collapse_labels: Query<(&PanelCollapseButtonLabel, &mut Text)>,
    mut close_query: Query<&mut Visibility>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    for (viewport, mut node, mut scroll_position) in &mut viewport_nodes {
        let Some(panel) = panels.panels.get(&viewport.panel_id) else {
            continue;
        };
        node.display = if panel.state.collapsed {
            Display::None
        } else {
            Display::Flex
        };
        node.overflow = if panel.state.scroll_enabled {
            Overflow::scroll_y()
        } else {
            Overflow::visible()
        };
        if panel.state.scroll_enabled {
            let available_height = (window.height() - panel.spec.margin_y * 2.0).max(0.0);
            let max_total = match panel.spec.scroll_policy {
                PanelScrollPolicy::AutoHalfScreen => available_height * 0.5,
                PanelScrollPolicy::FromTotalHeightPx(px) => px.max(PANEL_HEADER_HEIGHT),
                PanelScrollPolicy::Never => panel.natural_total_height.max(PANEL_HEADER_HEIGHT),
            };
            let max_content = (max_total - PANEL_HEADER_HEIGHT).max(0.0);
            node.height = Val::Px(max_content);
            node.min_height = Val::Px(max_content);
            node.max_height = Val::Px(max_content);
        } else {
            node.height = Val::Auto;
            node.min_height = Val::Auto;
            node.max_height = Val::Auto;
            scroll_position.offset_y = 0.0;
        }
    }

    for (label, mut text) in &mut collapse_labels {
        let Some(panel) = panels.panels.get(&label.panel_id) else {
            continue;
        };
        text.0 = if panel.state.collapsed { "+" } else { "-" }.to_string();
    }

    for panel in panels.panels.values() {
        if let Some(button_entity) = panel.close_button {
            if let Ok(mut visibility) = close_query.get_mut(button_entity) {
                *visibility = if panel.spec.controls.show_close {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
            }
        }
    }
}

#[derive(Clone, Copy)]
struct PanelScrollMetrics {
    viewport_height_px: f32,
    content_height_px: f32,
    max_scroll_logical: f32,
}

fn panel_scroll_metrics(computed: &ComputedNode) -> PanelScrollMetrics {
    let viewport_height_px = computed.size().y.max(0.0);
    let content_height_px = computed.content_size().y.max(viewport_height_px);
    let max_scroll_logical = ((content_height_px - viewport_height_px).max(0.0)
        * computed.inverse_scale_factor())
    .max(0.0);

    PanelScrollMetrics {
        viewport_height_px,
        content_height_px,
        max_scroll_logical,
    }
}

fn apply_panel_scrolling(
    mut mouse_wheel: EventReader<MouseWheel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    panels: Res<PanelManager>,
    mut viewport_nodes: Query<(&PanelContentViewport, &ComputedNode, &mut ScrollPosition)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Some(target_panel) = panels.topmost_scrollable_panel_at(cursor) else {
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

    for (viewport, computed, mut scroll_position) in &mut viewport_nodes {
        if viewport.panel_id != target_panel {
            continue;
        }
        let metrics = panel_scroll_metrics(computed);
        let max_scroll = metrics.max_scroll_logical;
        let next = (scroll_position.offset_y - delta).max(0.0);
        scroll_position.offset_y = next.min(max_scroll);
    }
}

fn sync_panel_scrollbar_visuals(
    panels: Res<PanelManager>,
    viewport_nodes: Query<(&PanelContentViewport, &ComputedNode, &ScrollPosition)>,
    track_sizes: Query<(&PanelScrollbarTrack, &ComputedNode)>,
    mut scrollbar_nodes: Query<
        (
            Option<&PanelScrollbarTrack>,
            Option<&PanelScrollbarThumb>,
            &mut Node,
            &mut Visibility,
        ),
        Or<(With<PanelScrollbarTrack>, With<PanelScrollbarThumb>)>,
    >,
) {
    let mut viewport_data = HashMap::new();
    for (viewport, computed, scroll_position) in &viewport_nodes {
        let metrics = panel_scroll_metrics(computed);
        viewport_data.insert(viewport.panel_id, (metrics, scroll_position.offset_y));
    }

    let mut track_height_by_panel = HashMap::new();
    for (track, computed) in &track_sizes {
        track_height_by_panel.insert(track.panel_id, computed.size().y.max(0.0));
    }

    for (maybe_track, maybe_thumb, mut node, mut visibility) in &mut scrollbar_nodes {
        if let Some(track) = maybe_track {
            let Some(panel) = panels.panels.get(&track.panel_id) else {
                *visibility = Visibility::Hidden;
                node.display = Display::None;
                continue;
            };
            let Some((metrics, _)) = viewport_data.get(&track.panel_id).copied() else {
                *visibility = Visibility::Hidden;
                node.display = Display::None;
                continue;
            };
            let max_scroll = metrics.max_scroll_logical;
            let show = panel.state.visible
                && panel.state.scroll_enabled
                && !panel.state.collapsed
                && max_scroll > 1.0;
            *visibility = if show {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
            node.display = if show { Display::Flex } else { Display::None };
            continue;
        }

        if let Some(thumb) = maybe_thumb {
            let Some(panel) = panels.panels.get(&thumb.panel_id) else {
                *visibility = Visibility::Hidden;
                continue;
            };
            let Some((metrics, offset_y)) = viewport_data.get(&thumb.panel_id).copied() else {
                *visibility = Visibility::Hidden;
                continue;
            };
            let track_h = track_height_by_panel
                .get(&thumb.panel_id)
                .copied()
                .unwrap_or(0.0);
            let max_scroll = metrics.max_scroll_logical;
            let show = panel.state.visible
                && panel.state.scroll_enabled
                && !panel.state.collapsed
                && max_scroll > 1.0
                && track_h > 1.0;
            if !show {
                *visibility = Visibility::Hidden;
                continue;
            }

            let ratio = (metrics.viewport_height_px / metrics.content_height_px).clamp(0.0, 1.0);
            let thumb_h = (ratio * track_h).clamp(
                SCROLLBAR_THUMB_MIN_HEIGHT,
                track_h.max(SCROLLBAR_THUMB_MIN_HEIGHT),
            );
            let travel = (track_h - thumb_h).max(0.0);
            let scroll_ratio = (offset_y / max_scroll).clamp(0.0, 1.0);
            let thumb_top = scroll_ratio * travel;

            node.top = Val::Px(thumb_top);
            node.height = Val::Px(thumb_h);
            *visibility = Visibility::Visible;
        }
    }
}

fn update_panel_hit_rects(
    mut panels: ResMut<PanelManager>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok(window) = windows.single() else {
        panels.hit_rects.clear();
        return;
    };

    let mut next = Vec::new();
    for (panel_id, panel) in panels.panels.iter() {
        if !panel.state.visible {
            continue;
        }
        let height = if panel.state.collapsed {
            PANEL_HEADER_HEIGHT
        } else {
            panel.layout_height.max(PANEL_HEADER_HEIGHT)
        };
        let (left, top) = match panel.spec.corner {
            PanelCorner::TopLeft => {
                let y = panel_anchor_offset(&panels, *panel_id);
                (panel.spec.margin_x, panel.spec.margin_y + y)
            }
            PanelCorner::TopRight => {
                let y = panel_anchor_offset(&panels, *panel_id);
                (
                    (window.width() - panel.spec.margin_x - panel.spec.width).max(0.0),
                    panel.spec.margin_y + y,
                )
            }
            PanelCorner::BottomLeft => {
                let y = panel_anchor_offset(&panels, *panel_id);
                (
                    panel.spec.margin_x,
                    (window.height() - panel.spec.margin_y - y - height).max(0.0),
                )
            }
            PanelCorner::BottomRight => {
                let y = panel_anchor_offset(&panels, *panel_id);
                (
                    (window.width() - panel.spec.margin_x - panel.spec.width).max(0.0),
                    (window.height() - panel.spec.margin_y - y - height).max(0.0),
                )
            }
        };

        next.push(PanelHitRect {
            panel_id: *panel_id,
            rect: PanelRect {
                left,
                top,
                width: panel.spec.width,
                height,
            },
            global_z: panel.state.opened_at as i32,
            scrollable: panel.state.scroll_enabled && !panel.state.collapsed,
        });
    }

    next.sort_by_key(|item| item.global_z);
    panels.hit_rects = next;
}

fn panel_anchor_offset(manager: &PanelManager, target: PanelId) -> f32 {
    let Some(target_panel) = manager.panels.get(&target) else {
        return 0.0;
    };
    if !target_panel.state.visible {
        return 0.0;
    }

    let mut ordered = manager
        .panels
        .iter()
        .filter_map(|(id, panel)| {
            (panel.state.visible && panel.spec.corner == target_panel.spec.corner)
                .then_some((*id, panel.state.opened_at))
        })
        .collect::<Vec<_>>();
    ordered.sort_by_key(|(_, opened_at)| *opened_at);

    let mut offset = 0.0;
    for (panel_id, _) in ordered {
        if panel_id == target {
            break;
        }
        if let Some(panel) = manager.panels.get(&panel_id) {
            let panel_height = if panel.state.collapsed {
                PANEL_HEADER_HEIGHT
            } else {
                panel.layout_height.max(PANEL_HEADER_HEIGHT)
            };
            offset += panel_height + panel.spec.stack_gap;
        }
    }
    offset
}

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
