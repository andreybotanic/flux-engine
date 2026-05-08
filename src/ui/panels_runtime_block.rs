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
    mut viewport_nodes: Query<(
        &PanelContentViewport,
        &mut Node,
        &mut ScrollPosition,
        &mut ScrollAreaViewport,
    )>,
    mut collapse_labels: Query<(&PanelCollapseButtonLabel, &mut Text)>,
    mut close_query: Query<&mut Visibility>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    for (viewport, mut node, mut scroll_position, mut scroll_area) in &mut viewport_nodes {
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
        scroll_area.enabled = panel.state.visible && panel.state.scroll_enabled && !panel.state.collapsed;
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
            rect: PanelRect {
                left,
                top,
                width: panel.spec.width,
                height,
            },
            global_z: panel.state.opened_at as i32,
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

