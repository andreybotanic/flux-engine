use bevy::prelude::*;

const COLLAPSIBLE_HEADER_HEIGHT: f32 = 30.0;
const COLLAPSIBLE_ARROW_COLLAPSED: &str = "v";
const COLLAPSIBLE_ARROW_EXPANDED: &str = "^";

/// Runtime configuration for a reusable collapsible content block.
#[derive(Clone, Debug)]
pub struct CollapsibleBlockConfig {
    /// Header title text. Can be empty.
    pub title: String,
    /// Initial collapsed state of the content.
    pub initial_collapsed: bool,
    /// Background color of the whole block.
    pub background: Color,
    /// Background color of the header row.
    pub header_background: Color,
    /// Optional dropdown-style arrow image shown at the end of the header.
    pub arrow_icon: Option<Handle<Image>>,
    /// Vertical gap between content rows.
    pub content_row_gap_px: f32,
    /// Padding inside the content area.
    pub content_padding: UiRect,
}

impl CollapsibleBlockConfig {
    /// Creates a collapsible block config with default visual styling.
    pub fn new(title: impl Into<String>, initial_collapsed: bool) -> Self {
        Self {
            title: title.into(),
            initial_collapsed,
            ..Default::default()
        }
    }
}

impl Default for CollapsibleBlockConfig {
    fn default() -> Self {
        Self {
            title: String::new(),
            initial_collapsed: false,
            background: crate::ui::palette::PANEL_BG,
            header_background: crate::ui::palette::PANEL_HEADER_BG,
            arrow_icon: None,
            content_row_gap_px: 8.0,
            content_padding: UiRect::all(Val::Px(8.0)),
        }
    }
}

/// Spawns a collapsible block that can host arbitrary nested UI content.
pub fn spawn_collapsible_block(
    parent: &mut ChildSpawnerCommands,
    config: CollapsibleBlockConfig,
    build_content: impl FnOnce(&mut ChildSpawnerCommands),
) -> Entity {
    let mut root = parent.spawn((
        Node {
            width: Val::Percent(100.0),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundColor(config.background),
        CollapsibleBlockRoot {
            collapsed: config.initial_collapsed,
        },
    ));

    let root_id = root.id();
    root.with_children(|block| {
        block
            .spawn((
                Button,
                Node {
                    width: Val::Percent(100.0),
                    min_height: Val::Px(COLLAPSIBLE_HEADER_HEIGHT),
                    max_height: Val::Px(COLLAPSIBLE_HEADER_HEIGHT),
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(config.header_background),
                CollapsibleBlockHeaderButton { root: root_id },
            ))
            .with_children(|header| {
                header.spawn((
                    Text::new(config.title.clone()),
                    TextFont::from_font_size(13.0),
                    TextColor(crate::ui::palette::TEXT_PRIMARY),
                ));

                if let Some(arrow_icon) = config.arrow_icon.clone() {
                    header.spawn((
                        ImageNode::new(arrow_icon),
                        Node {
                            width: Val::Px(10.0),
                            height: Val::Px(6.0),
                            ..default()
                        },
                        CollapsibleBlockToggleArrowImage { root: root_id },
                    ));
                } else {
                    header.spawn((
                        Text::new(if config.initial_collapsed {
                            COLLAPSIBLE_ARROW_COLLAPSED
                        } else {
                            COLLAPSIBLE_ARROW_EXPANDED
                        }),
                        TextFont::from_font_size(14.0),
                        TextColor(crate::ui::palette::TEXT_PRIMARY),
                        CollapsibleBlockToggleLabel { root: root_id },
                    ));
                }
            });

        block
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    display: if config.initial_collapsed {
                        Display::None
                    } else {
                        Display::Flex
                    },
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(config.content_row_gap_px),
                    padding: config.content_padding,
                    border: UiRect::new(Val::Px(1.0), Val::Px(1.0), Val::Px(0.0), Val::Px(1.0)),
                    ..default()
                },
                BorderColor(config.header_background),
                CollapsibleBlockContent { root: root_id },
            ))
            .with_children(build_content);
    });

    root_id
}

#[derive(Component, Clone, Copy)]
struct CollapsibleBlockRoot {
    collapsed: bool,
}

#[derive(Component, Clone, Copy)]
struct CollapsibleBlockHeaderButton {
    root: Entity,
}

#[derive(Component, Clone, Copy)]
struct CollapsibleBlockToggleLabel {
    root: Entity,
}

#[derive(Component, Clone, Copy)]
struct CollapsibleBlockToggleArrowImage {
    root: Entity,
}

#[derive(Component, Clone, Copy)]
struct CollapsibleBlockContent {
    root: Entity,
}

fn handle_collapsible_block_header_clicks(
    mut interactions: Query<
        (&Interaction, &CollapsibleBlockHeaderButton),
        (Changed<Interaction>, With<Button>),
    >,
    mut roots: Query<&mut CollapsibleBlockRoot>,
) {
    for (interaction, header) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let Ok(mut root) = roots.get_mut(header.root) else {
            continue;
        };
        root.collapsed = !root.collapsed;
    }
}

fn sync_collapsible_block_visuals(
    roots: Query<(Entity, &CollapsibleBlockRoot), Changed<CollapsibleBlockRoot>>,
    mut contents: Query<(&CollapsibleBlockContent, &mut Node)>,
    mut labels: Query<(&CollapsibleBlockToggleLabel, &mut Text)>,
    mut arrows: Query<(&CollapsibleBlockToggleArrowImage, &mut ImageNode)>,
) {
    for (root_entity, root) in &roots {
        for (content, mut node) in &mut contents {
            if content.root == root_entity {
                node.display = if root.collapsed {
                    Display::None
                } else {
                    Display::Flex
                };
            }
        }
        for (label, mut text) in &mut labels {
            if label.root == root_entity {
                text.0 = if root.collapsed {
                    COLLAPSIBLE_ARROW_COLLAPSED
                } else {
                    COLLAPSIBLE_ARROW_EXPANDED
                }
                .to_string();
            }
        }
        for (arrow, mut image) in &mut arrows {
            if arrow.root == root_entity {
                image.flip_y = !root.collapsed;
            }
        }
    }
}

/// Stores `CollapsibleBlockPlugin` state.
pub struct CollapsibleBlockPlugin;

impl Plugin for CollapsibleBlockPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                handle_collapsible_block_header_clicks,
                sync_collapsible_block_visuals,
            )
                .chain(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        spawn_collapsible_block, CollapsibleBlockConfig, CollapsibleBlockContent,
        CollapsibleBlockHeaderButton, CollapsibleBlockPlugin, CollapsibleBlockRoot,
        CollapsibleBlockToggleArrowImage, CollapsibleBlockToggleLabel,
    };
    use bevy::prelude::*;

    fn setup_app() -> App {
        let mut app = App::new();
        app.add_plugins(CollapsibleBlockPlugin);
        app
    }

    fn spawn_test_block(app: &mut App, initial_collapsed: bool) -> Entity {
        let root = app
            .world_mut()
            .spawn(CollapsibleBlockRoot {
                collapsed: initial_collapsed,
            })
            .id();
        app.world_mut().spawn((
            Button,
            Interaction::None,
            CollapsibleBlockHeaderButton { root },
        ));
        app.world_mut().spawn((
            Text::new(if initial_collapsed {
                super::COLLAPSIBLE_ARROW_COLLAPSED
            } else {
                super::COLLAPSIBLE_ARROW_EXPANDED
            }),
            CollapsibleBlockToggleLabel { root },
        ));
        app.world_mut().spawn((
            Node {
                display: if initial_collapsed {
                    Display::None
                } else {
                    Display::Flex
                },
                ..default()
            },
            CollapsibleBlockContent { root },
        ));
        root
    }

    #[test]
    fn header_click_flips_collapsed_state() {
        let mut app = setup_app();
        let _ui_root = spawn_test_block(&mut app, false);
        app.update();

        let header = app
            .world_mut()
            .query_filtered::<Entity, With<CollapsibleBlockHeaderButton>>()
            .single(app.world())
            .expect("test block should spawn one header button");
        app.world_mut()
            .entity_mut(header)
            .insert(Interaction::Pressed);
        app.update();

        let root = app
            .world_mut()
            .query::<&CollapsibleBlockRoot>()
            .single(app.world())
            .expect("test block should spawn one collapsible root");
        assert!(root.collapsed);
    }

    #[test]
    fn toggle_label_uses_triangle_arrow_by_state() {
        let mut app = setup_app();
        let _ui_root = spawn_test_block(&mut app, false);
        app.update();

        let label_text = app
            .world_mut()
            .query_filtered::<&Text, With<CollapsibleBlockToggleLabel>>()
            .single(app.world())
            .expect("test block should spawn one toggle label");
        assert_eq!(label_text.0, super::COLLAPSIBLE_ARROW_EXPANDED);

        let header = app
            .world_mut()
            .query_filtered::<Entity, With<CollapsibleBlockHeaderButton>>()
            .single(app.world())
            .expect("test block should spawn one header button");
        app.world_mut()
            .entity_mut(header)
            .insert(Interaction::Pressed);
        app.update();

        let label_text = app
            .world_mut()
            .query_filtered::<&Text, With<CollapsibleBlockToggleLabel>>()
            .single(app.world())
            .expect("test block should spawn one toggle label");
        assert_eq!(label_text.0, super::COLLAPSIBLE_ARROW_COLLAPSED);
    }

    #[test]
    fn content_visibility_tracks_collapsed_state() {
        let mut app = setup_app();
        let _ui_root = spawn_test_block(&mut app, false);
        app.update();

        let content_display = app
            .world_mut()
            .query_filtered::<&Node, With<CollapsibleBlockContent>>()
            .single(app.world())
            .expect("test block should spawn one content node")
            .display;
        assert_eq!(content_display, Display::Flex);

        let header = app
            .world_mut()
            .query_filtered::<Entity, With<CollapsibleBlockHeaderButton>>()
            .single(app.world())
            .expect("test block should spawn one header button");
        app.world_mut()
            .entity_mut(header)
            .insert(Interaction::Pressed);
        app.update();

        let content_display = app
            .world_mut()
            .query_filtered::<&Node, With<CollapsibleBlockContent>>()
            .single(app.world())
            .expect("test block should spawn one content node")
            .display;
        assert_eq!(content_display, Display::None);
    }

    #[test]
    fn content_border_uses_header_color_and_root_has_no_frame_border() {
        let mut app = setup_app();
        let header_color = Color::srgb(0.2, 0.3, 0.4);
        app.add_systems(Startup, move |mut commands: Commands| {
            commands.spawn(Node::default()).with_children(|children| {
                let mut config = CollapsibleBlockConfig::new("Metrics", false);
                config.header_background = header_color;
                let _root = spawn_collapsible_block(children, config, |_content| {});
            });
        });
        app.update();

        let root = app
            .world_mut()
            .query_filtered::<Entity, With<CollapsibleBlockRoot>>()
            .single(app.world())
            .expect("test block should spawn one collapsible root");
        let root_node = app
            .world()
            .get::<Node>(root)
            .expect("root should have ui node");
        assert_ne!(root_node.border.left, Val::Px(1.0));
        assert_ne!(root_node.border.right, Val::Px(1.0));
        assert_ne!(root_node.border.top, Val::Px(1.0));
        assert_ne!(root_node.border.bottom, Val::Px(1.0));

        let content_entity = app
            .world_mut()
            .query_filtered::<Entity, With<CollapsibleBlockContent>>()
            .single(app.world())
            .expect("test block should spawn one content node");
        let content_border_color = app
            .world()
            .get::<BorderColor>(content_entity)
            .expect("content should have border color")
            .0;
        assert_eq!(
            content_border_color, header_color,
            "content border color should match header row color"
        );

        let content_node = app
            .world()
            .get::<Node>(content_entity)
            .expect("content should have ui node");
        assert_eq!(content_node.border.left, Val::Px(1.0));
        assert_eq!(content_node.border.right, Val::Px(1.0));
        assert_eq!(content_node.border.top, Val::Px(0.0));
        assert_eq!(content_node.border.bottom, Val::Px(1.0));
    }

    #[test]
    fn image_arrow_flips_like_select_icon() {
        let mut app = setup_app();
        let root = app
            .world_mut()
            .spawn(CollapsibleBlockRoot { collapsed: true })
            .id();
        app.world_mut().spawn((
            Button,
            Interaction::None,
            CollapsibleBlockHeaderButton { root },
        ));
        app.world_mut().spawn((
            ImageNode::new(Handle::<Image>::default()),
            CollapsibleBlockToggleArrowImage { root },
        ));
        app.update();

        let arrow_initial = app
            .world_mut()
            .query_filtered::<&ImageNode, With<CollapsibleBlockToggleArrowImage>>()
            .single(app.world())
            .expect("test block should spawn one image arrow");
        assert!(!arrow_initial.flip_y);

        let header = app
            .world_mut()
            .query_filtered::<Entity, With<CollapsibleBlockHeaderButton>>()
            .single(app.world())
            .expect("test block should spawn one header button");
        app.world_mut()
            .entity_mut(header)
            .insert(Interaction::Pressed);
        app.update();

        let arrow_expanded = app
            .world_mut()
            .query_filtered::<&ImageNode, With<CollapsibleBlockToggleArrowImage>>()
            .single(app.world())
            .expect("test block should spawn one image arrow");
        assert!(arrow_expanded.flip_y);
    }
}
