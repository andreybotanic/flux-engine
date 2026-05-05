/// Stores `TextInputPlugin` state.
pub struct TextInputPlugin;

impl Plugin for TextInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TextInputCaretBlink>();
        app.add_systems(
            Update,
            (
                ensure_text_input_caret,
                focus_text_input_on_click,
                handle_text_input_keyboard,
                sync_text_input_display,
                sync_text_input_style,
                sync_text_input_caret,
            )
                .chain(),
        );
    }
}

fn ensure_text_input_caret(
    mut commands: Commands,
    inputs: Query<Entity, (Added<TextInputField>, With<Button>)>,
) {
    for input_entity in &inputs {
        commands.entity(input_entity).with_children(|parent| {
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(CARET_WIDTH_PX),
                    height: Val::Px(CARET_MIN_HEIGHT_PX),
                    ..default()
                },
                BackgroundColor(crate::ui::palette::TEXT_ON_DARK),
                Visibility::Hidden,
                TextInputCaret,
            ));
        });
    }
}

fn focus_text_input_on_click(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut query_set: ParamSet<(
        Query<
            (Entity, &Interaction, &RelativeCursorPosition),
            (Changed<Interaction>, With<TextInputField>),
        >,
        Query<(Entity, &mut TextInputField, &ComputedNode)>,
    )>,
    display_layout_query: Query<(&bevy::text::ComputedTextBlock, &ChildOf), With<TextInputDisplay>>,
) {
    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let mut clicked: Option<(Entity, Option<Vec2>)> = None;

    for (entity, interaction, relative_cursor) in &mut query_set.p0() {
        if *interaction == Interaction::Pressed {
            clicked = Some((entity, relative_cursor.normalized));
            break;
        }
    }

    for (entity, mut field, input_node) in &mut query_set.p1() {
        if let Some((clicked_entity, normalized)) = clicked {
            if entity == clicked_entity {
                field.focused = true;
                let input_width = input_node.size().x.max(1.0);
                let input_height = input_node.size().y.max(1.0);
                let content_left = input_node.content_inset().left;
                let content_top = input_node.content_inset().top;
                let relative_x = normalized.map(|p| p.x.clamp(0.0, 1.0) * input_width);
                let relative_y = normalized.map(|p| p.y.clamp(0.0, 1.0) * input_height);
                let click_x = (relative_x.unwrap_or(input_width) - content_left).max(0.0);
                let click_y = (relative_y.unwrap_or(0.0) - content_top).max(0.0);
                let text_layout = display_layout_query
                    .iter()
                    .find(|(_, child_of)| child_of.parent() == entity)
                    .map(|(layout, _)| layout);
                field.cursor =
                    cursor_index_from_click_position_exact(&field.text, click_x, click_y, text_layout);

                field.clamp_cursor();
            } else {
                field.focused = false;
            }
        } else {
            field.focused = false;
        }
    }
}

fn handle_text_input_keyboard(
    mut keyboard_events: EventReader<KeyboardInput>,
    mut inputs: Query<&mut TextInputField>,
    mut blink: ResMut<TextInputCaretBlink>,
) {
    let mut target: Option<Mut<TextInputField>> = None;
    for field in &mut inputs {
        if field.focused {
            target = Some(field);
            break;
        }
    }

    let Some(mut field) = target else {
        return;
    };

    field.clamp_cursor();

    for event in keyboard_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }

        let mut edited = false;
        match &event.logical_key {
            Key::ArrowLeft => {
                field.move_left();
                edited = true;
            }
            Key::ArrowRight => {
                field.move_right();
                edited = true;
            }
            Key::Home => {
                field.cursor = 0;
                edited = true;
            }
            Key::End => {
                field.cursor = field.char_count();
                edited = true;
            }
            Key::Backspace => {
                field.backspace();
                edited = true;
            }
            Key::Delete => {
                field.delete();
                edited = true;
            }
            Key::Enter | Key::Escape => {
                field.focused = false;
                edited = true;
            }
            _ => {
                if let Some(text) = &event.text {
                    for ch in text.chars() {
                        field.insert_char(ch);
                        edited = true;
                    }
                } else if let Key::Character(chars) = &event.logical_key {
                    for ch in chars.chars() {
                        field.insert_char(ch);
                        edited = true;
                    }
                }
            }
        }

        if edited {
            wake_caret(&mut blink);
        }
    }
}

fn wake_caret(blink: &mut TextInputCaretBlink) {
    blink.visible = true;
    blink.timer.reset();
}

fn sync_text_input_display(
    parent_query: Query<&ChildOf>,
    inputs: Query<&TextInputField>,
    mut text_query: Query<(Entity, &mut Text), With<TextInputDisplay>>,
) {
    for (text_entity, mut text) in &mut text_query {
        let Ok(parent) = parent_query.get(text_entity) else {
            continue;
        };

        let Ok(field) = inputs.get(parent.parent()) else {
            continue;
        };

        text.0 = field.text.clone();
    }
}

fn sync_text_input_style(
    inputs: Query<(&TextInputField, &TextInputStyle)>,
    mut bg_query: Query<(&mut BackgroundColor, Entity), With<Button>>,
) {
    for (mut bg, entity) in &mut bg_query {
        let Ok((field, style)) = inputs.get(entity) else {
            continue;
        };

        bg.0 = if field.focused {
            style.focused_bg
        } else {
            style.idle_bg
        };
    }
}

fn sync_text_input_caret(
    time: Res<Time>,
    mut blink: ResMut<TextInputCaretBlink>,
    input_query: Query<(&TextInputField, &ComputedNode)>,
    display_layout_query: Query<(&bevy::text::ComputedTextBlock, &ChildOf), With<TextInputDisplay>>,
    mut caret_query: Query<
        (&mut Node, &mut Visibility, &ChildOf, &mut BackgroundColor),
        With<TextInputCaret>,
    >,
) {
    blink.timer.tick(time.delta());
    if blink.timer.just_finished() {
        blink.visible = !blink.visible;
    }

    for (mut caret_node, mut caret_visibility, child_of, mut caret_color) in &mut caret_query {
        let input_entity = child_of.parent();
        let Ok((field, input_node)) = input_query.get(input_entity) else {
            *caret_visibility = Visibility::Hidden;
            continue;
        };

        if !field.focused || !blink.visible {
            *caret_visibility = Visibility::Hidden;
            continue;
        }

        let content_inset = input_node.content_inset();
        let text_origin_x = content_inset.left;
        let text_origin_y = content_inset.top;
        let content_height =
            (input_node.size().y - content_inset.top - content_inset.bottom).max(0.0);
        let caret_height = (content_height * CARET_HEIGHT_FACTOR).max(CARET_MIN_HEIGHT_PX);
        let caret_top = text_origin_y + ((content_height - caret_height) * 0.5).max(0.0);

        let text_layout = display_layout_query
            .iter()
            .find(|(_, layout_child_of)| layout_child_of.parent() == input_entity)
            .map(|(layout, _)| layout);
        let caret_x = text_origin_x + caret_x_from_cursor_exact(&field.text, field.cursor, text_layout);

        caret_node.left =
            Val::Px((caret_x - (CARET_WIDTH_PX * 0.5)).max(text_origin_x - CARET_SIDE_PADDING_PX));
        caret_node.top = Val::Px(caret_top);
        caret_node.width = Val::Px(CARET_WIDTH_PX);
        caret_node.height = Val::Px(caret_height);
        caret_color.0 = crate::ui::palette::TEXT_ON_DARK;
        *caret_visibility = Visibility::Visible;
    }
}

