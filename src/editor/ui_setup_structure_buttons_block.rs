fn spawn_gas_tool_panel_content(
    parent: &mut ChildSpawnerCommands,
    gas_options: &[String],
    select_arrow: Handle<Image>,
) {
    spawn_select_field(
        parent,
        &SelectFieldConfig {
            id: GAS_SELECT_ADD_ID,
            options: gas_options.to_vec(),
            selected: 0,
        },
        "Gas:",
        select_arrow,
    );

    parent
        .spawn((Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(8.0),
            align_items: AlignItems::Center,
            ..default()
        },))
        .with_children(|row| {
            row.spawn((
                Text::new("Amount:"),
                TextFont::from_font_size(13.0),
                TextColor(crate::ui::palette::TEXT_PRIMARY),
            ));

            row.spawn((
                Button,
                Node {
                    min_width: Val::Px(112.0),
                    height: Val::Px(30.0),
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(0.0)),
                    ..default()
                },
                BackgroundColor(BUTTON_IDLE),
                TextInputField::new_u32(100, 1, 1_000_000, 7),
                TextInputStyle {
                    idle_bg: BUTTON_IDLE,
                    focused_bg: INPUT_FOCUSED,
                },
                bevy::ui::RelativeCursorPosition::default(),
                GasAmountInputField,
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new("100"),
                    TextFont::from_font_size(13.0),
                    TextColor(crate::ui::palette::TEXT_PRIMARY),
                    TextInputDisplay,
                ));
            });
        });

    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(190.0),
                height: Val::Px(32.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BUTTON_IDLE),
            EditorUiAction::ToggleReplace,
        ))
        .with_children(|button| {
            button.spawn((
                Text::new("Replace: Off"),
                TextFont::from_font_size(13.0),
                TextColor(crate::ui::palette::TEXT_PRIMARY),
                GasReplaceLabel,
            ));
        });
}

fn spawn_structure_tool_panel_content(
    parent: &mut ChildSpawnerCommands,
    gas_options: &[String],
    select_arrow: Handle<Image>,
) {
    parent.spawn((
        Text::new("No structure selected"),
        TextFont::from_font_size(13.0),
        TextColor(crate::ui::palette::TEXT_PRIMARY),
        StructureModeLabel,
    ));

    parent
        .spawn((
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
            StructureSourceSection,
        ))
        .with_children(|source| {
            spawn_select_field(
                source,
                &SelectFieldConfig {
                    id: GAS_SELECT_SOURCE_ID,
                    options: gas_options.to_vec(),
                    selected: 0,
                },
                "Source gas:",
                select_arrow,
            );
            source
                .spawn((Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(8.0),
                    align_items: AlignItems::Center,
                    ..default()
                },))
                .with_children(|row| {
                    row.spawn((
                        Text::new("Amount:"),
                        TextFont::from_font_size(13.0),
                        TextColor(crate::ui::palette::TEXT_PRIMARY),
                    ));
                    row.spawn((
                        Button,
                        Node {
                            min_width: Val::Px(112.0),
                            height: Val::Px(30.0),
                            justify_content: JustifyContent::FlexStart,
                            align_items: AlignItems::Center,
                            padding: UiRect::axes(Val::Px(8.0), Val::Px(0.0)),
                            ..default()
                        },
                        BackgroundColor(BUTTON_IDLE),
                        TextInputField::new_u32(100, 1, 1_000_000, 7),
                        TextInputStyle {
                            idle_bg: BUTTON_IDLE,
                            focused_bg: INPUT_FOCUSED,
                        },
                        bevy::ui::RelativeCursorPosition::default(),
                        SourceAmountInputField,
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new("100"),
                            TextFont::from_font_size(13.0),
                            TextColor(crate::ui::palette::TEXT_PRIMARY),
                            TextInputDisplay,
                        ));
                    });
                });
        });

    parent
        .spawn((
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
            StructureSinkSection,
        ))
        .with_children(|sink| {
            sink.spawn((
                Text::new("Sink amount per step"),
                TextFont::from_font_size(13.0),
                TextColor(crate::ui::palette::TEXT_PRIMARY),
            ));
            sink.spawn((
                Button,
                Node {
                    min_width: Val::Px(112.0),
                    height: Val::Px(30.0),
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(0.0)),
                    ..default()
                },
                BackgroundColor(BUTTON_IDLE),
                TextInputField::new_u32(100, 1, 1_000_000, 7),
                TextInputStyle {
                    idle_bg: BUTTON_IDLE,
                    focused_bg: INPUT_FOCUSED,
                },
                bevy::ui::RelativeCursorPosition::default(),
                SinkAmountInputField,
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new("100"),
                    TextFont::from_font_size(13.0),
                    TextColor(crate::ui::palette::TEXT_PRIMARY),
                    TextInputDisplay,
                ));
            });
        });
}

fn spawn_cells_variant_panel_content(
    parent: &mut ChildSpawnerCommands,
    variants: &[(String, CellMaterial, Handle<Image>)],
) {
    for row_variants in variants.chunks(TOOL_VARIANT_PANEL_COLUMNS) {
        parent
            .spawn((Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(TOOL_VARIANT_BUTTON_GAP),
                ..default()
            },))
            .with_children(|row| {
                for (label, material, icon) in row_variants.iter() {
                    spawn_cell_material_button(
                        row,
                        label.clone(),
                        *material,
                        icon.clone(),
                        TOOL_VARIANT_BUTTON_SIZE,
                        TOOL_VARIANT_ICON_SIZE,
                    );
                }
            });
    }
}

fn spawn_gases_variant_panel_content(
    parent: &mut ChildSpawnerCommands,
    variants: &[(String, PipeToolKind, Handle<Image>)],
) {
    for row_variants in variants.chunks(TOOL_VARIANT_PANEL_COLUMNS) {
        parent
            .spawn((Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(TOOL_VARIANT_BUTTON_GAP),
                ..default()
            },))
            .with_children(|row| {
                for (label, pipe_tool, icon) in row_variants.iter() {
                    spawn_pipe_tool_button(
                        row,
                        label.clone(),
                        *pipe_tool,
                        icon.clone(),
                        TOOL_VARIANT_BUTTON_SIZE,
                        TOOL_VARIANT_ICON_SIZE,
                    );
                }
            });
    }
}

fn spawn_entity_category_button(
    parent: &mut ChildSpawnerCommands,
    label: String,
    category_id: ContentId,
    icon: Handle<Image>,
    button_size: f32,
    icon_size: f32,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(button_size),
                height: Val::Px(button_size),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BUTTON_IDLE),
            EditorUiAction::SelectEntityCategory(category_id),
            ToolButtonMeta {
                label: label.clone(),
            },
        ))
        .with_children(|button| {
            button.spawn((
                ImageNode::new(icon),
                Node {
                    width: Val::Px(icon_size),
                    height: Val::Px(icon_size),
                    ..default()
                },
            ));
        });
}

fn spawn_tool_button(
    parent: &mut ChildSpawnerCommands,
    label: impl Into<String>,
    tool: EditorTool,
    icon: Handle<Image>,
    button_size: f32,
    icon_size: f32,
) {
    let label = label.into();
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(button_size),
                height: Val::Px(button_size),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BUTTON_IDLE),
            EditorUiAction::SelectTool(tool),
            ToolButtonMeta {
                label: label.clone(),
            },
        ))
        .with_children(|button| {
            button.spawn((
                ImageNode::new(icon),
                Node {
                    width: Val::Px(icon_size),
                    height: Val::Px(icon_size),
                    ..default()
                },
            ));
        });
}

fn spawn_cell_material_button(
    parent: &mut ChildSpawnerCommands,
    label: String,
    material: CellMaterial,
    icon: Handle<Image>,
    button_size: f32,
    icon_size: f32,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(button_size),
                height: Val::Px(button_size),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BUTTON_IDLE),
            EditorUiAction::SelectCellMaterial(material),
            ToolButtonMeta {
                label: label.clone(),
            },
        ))
        .with_children(|button| {
            button.spawn((
                ImageNode::new(icon),
                Node {
                    width: Val::Px(icon_size),
                    height: Val::Px(icon_size),
                    ..default()
                },
            ));
        });
}

fn spawn_pipe_tool_button(
    parent: &mut ChildSpawnerCommands,
    label: String,
    pipe_tool: PipeToolKind,
    icon: Handle<Image>,
    button_size: f32,
    icon_size: f32,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(button_size),
                height: Val::Px(button_size),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BUTTON_IDLE),
            EditorUiAction::SelectPipeTool(pipe_tool),
            ToolButtonMeta {
                label: label.clone(),
            },
        ))
        .with_children(|button| {
            button.spawn((
                ImageNode::new(icon),
                Node {
                    width: Val::Px(icon_size),
                    height: Val::Px(icon_size),
                    ..default()
                },
            ));
        });
}

