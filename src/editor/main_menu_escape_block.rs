fn clear_active_tool_state(
    selection_drag: &mut ResMut<SelectionDragState>,
    brush_drag: &mut ResMut<BrushDragState>,
) {
    selection_drag.active = false;
    selection_drag.start = None;
    selection_drag.current = None;
    brush_drag.active = false;
    brush_drag.last_cell = None;
}

fn escape_action(
    menu_mode: MainMenuMode,
    has_selected_tool: bool,
    has_structure_editor: bool,
    has_world: bool,
) -> EscAction {
    match menu_mode {
        MainMenuMode::Main => EscAction::Ignore,
        MainMenuMode::InGame => EscAction::CloseMenuKeepPaused,
        MainMenuMode::Hidden => {
            if !has_world {
                EscAction::Ignore
            } else if has_structure_editor {
                EscAction::CloseStructureEditor
            } else if has_selected_tool {
                EscAction::ClearSelectedTool
            } else {
                EscAction::OpenMenuAndPause
            }
        }
    }
}

fn handle_escape_and_main_menu(
    keys: Res<ButtonInput<KeyCode>>,
    mut active_tool: ResMut<ActiveEditorTool>,
    mut main_menu: ResMut<MainMenuState>,
    mut menu_ui: ResMut<MainMenuUiState>,
    world_load_state: Res<WorldLoadState>,
    mut structure_edit: ResMut<StructureEditState>,
    mut control: ResMut<crate::simulation::SimulationControl>,
    mut input_fields: Query<&mut TextInputField>,
    mut selection_drag: ResMut<SelectionDragState>,
    mut brush_drag: ResMut<BrushDragState>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }

    let mut had_focused_input = false;
    for mut field in &mut input_fields {
        if field.focused {
            field.focused = false;
            had_focused_input = true;
        }
    }
    if had_focused_input {
        return;
    }

    match escape_action(
        menu_ui.mode,
        active_tool.selected.is_some(),
        structure_edit.selected_cell.is_some(),
        world_load_state.has_world,
    ) {
        EscAction::CloseMenuKeepPaused => {
            menu_ui.mode = MainMenuMode::Hidden;
            menu_ui.screen = MainMenuScreen::Root;
            main_menu.open = false;
        }
        EscAction::CloseStructureEditor => {
            structure_edit.selected_cell = None;
        }
        EscAction::ClearSelectedTool => {
            active_tool.selected = None;
            clear_active_tool_state(&mut selection_drag, &mut brush_drag);
        }
        EscAction::OpenMenuAndPause => {
            control.paused = true;
            menu_ui.mode = MainMenuMode::InGame;
            menu_ui.screen = MainMenuScreen::Root;
            main_menu.open = true;
        }
        EscAction::Ignore => {}
    }
}

