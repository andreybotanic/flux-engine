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
    rect: PanelRect,
    global_z: i32,
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
/// Stores `PanelManager` state.
pub struct PanelManager {
    panels: HashMap<PanelId, ManagedPanel>,
    hit_rects: Vec<PanelHitRect>,
}

impl PanelManager {
/// Runs `spawn_panel` logic.
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
            PanelRoot {
                _panel_id: panel_id,
            },
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

            let viewport = root
                .spawn((
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
                    RelativeCursorPosition::default(),
                    ScrollAreaViewport::panel(0),
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
                            PanelContentInner {
                                _panel_id: panel_id,
                            },
                        ))
                        .with_children(build_content)
                        .id();
                    content_inner_entity = Some(inner);
                })
                .id();
            spawn_scroll_area_scrollbar_with_style(
                root,
                viewport,
                ScrollAreaScrollbarStyle {
                    right_px: 2.0,
                    top_px: PANEL_HEADER_HEIGHT + PANEL_CONTENT_PADDING_Y,
                    bottom_px: PANEL_CONTENT_PADDING_Y,
                    width_px: 6.0,
                },
            );
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

/// Runs `set_visible` logic.
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

/// Runs `is_cursor_over_any_panel` logic.
    pub fn is_cursor_over_any_panel(&self, cursor: Vec2) -> bool {
        self.hit_rects.iter().any(|item| item.rect.contains(cursor))
    }

}

#[derive(Component, Clone, Copy)]
struct PanelRoot {
    _panel_id: PanelId,
}

#[derive(Component, Clone, Copy)]
struct PanelContentViewport {
    panel_id: PanelId,
}

#[derive(Component, Clone, Copy)]
struct PanelContentInner {
    _panel_id: PanelId,
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
/// Stores `PanelHeaderActionEvent` state.
pub struct PanelHeaderActionEvent {
    pub panel_id: PanelId,
    pub action_id: PanelHeaderActionId,
}

/// Stores `PanelPlugin` state.
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
                    update_panel_hit_rects,
                )
                    .chain(),
            );
    }
}

