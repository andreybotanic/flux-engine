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
                TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
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
                    TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
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
                TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
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
        TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
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
                        TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
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
                            TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
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
                TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
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
                    TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
                    TextInputDisplay,
                ));
            });
        });
}

fn spawn_tool_button(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    tool: EditorTool,
    icon: Handle<Image>,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(TOOL_BUTTON_SIZE),
                height: Val::Px(TOOL_BUTTON_SIZE),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BUTTON_IDLE),
            EditorUiAction::SelectTool(tool),
            ToolButtonMeta { label },
        ))
        .with_children(|button| {
            button.spawn((
                ImageNode::new(icon),
                Node {
                    width: Val::Px(TOOL_ICON_SIZE),
                    height: Val::Px(TOOL_ICON_SIZE),
                    ..default()
                },
            ));
        });
}

fn spawn_cell_material_button(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    material: CellMaterial,
    icon: Handle<Image>,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(TOOL_BUTTON_SIZE),
                height: Val::Px(TOOL_BUTTON_SIZE),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BUTTON_IDLE),
            EditorUiAction::SelectCellMaterial(material),
            ToolButtonMeta { label },
        ))
        .with_children(|button| {
            button.spawn((
                ImageNode::new(icon),
                Node {
                    width: Val::Px(TOOL_ICON_SIZE),
                    height: Val::Px(TOOL_ICON_SIZE),
                    ..default()
                },
            ));
        });
}

