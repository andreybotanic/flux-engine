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
    scroll_block: Res<crate::ui::scroll_area::UiScrollBlockState>,
    mut viewport_nodes: Query<(&PanelContentViewport, &ComputedNode, &mut ScrollPosition)>,
) {
    if scroll_block.block_panel_scrolling {
        return;
    }
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

