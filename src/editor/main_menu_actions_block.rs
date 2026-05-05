fn handle_main_menu_actions(
    mut interactions: Query<
        (&Interaction, &MainMenuActionButton, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut main_menu: ResMut<MainMenuState>,
    mut menu_ui: ResMut<MainMenuUiState>,
    mut control: ResMut<SimulationControl>,
    gas_registry: Res<GasRegistry>,
    mut world: ResMut<WorldGrid>,
    mut structures: ResMut<GasStructureGrid>,
    mut gas: ResMut<GasField>,
    mut step: ResMut<SimulationStep>,
    mut save_session: ResMut<SaveSessionState>,
    mut world_load_state: ResMut<WorldLoadState>,
    save_name_input: Single<&TextInputField, With<MainMenuSaveNameInputField>>,
    mut world_changed: EventWriter<WorldCellChanged>,
    mut exit_writer: EventWriter<AppExit>,
) {
    if !main_menu.open {
        return;
    }

    for (interaction, action_button, mut bg) in &mut interactions {
        match *interaction {
            Interaction::Pressed | Interaction::Hovered => bg.0 = MODAL_BUTTON_HOVER,
            Interaction::None => bg.0 = MODAL_BUTTON_BG,
        }
        if *interaction != Interaction::Pressed {
            continue;
        }

        match &action_button.0 {
            MainMenuButtonAction::Continue => {
                if menu_ui.mode != MainMenuMode::InGame || !world_load_state.has_world {
                    continue;
                }
                menu_ui.mode = MainMenuMode::Hidden;
                menu_ui.screen = MainMenuScreen::Root;
                menu_ui.confirm_state = None;
                menu_ui.post_save_action = None;
                main_menu.open = false;
                menu_ui.status_text.clear();
            }
            MainMenuButtonAction::NewGame => {
                let state = new_game_snapshot(&gas_registry);
                match apply_runtime_world_state(
                    state,
                    &mut world,
                    &mut structures,
                    &mut gas,
                    &mut step,
                    &mut world_changed,
                ) {
                    Ok(_) => {
                        control.paused = true;
                        save_session.mark_persisted(step.0, None);
                        world_load_state.has_world = true;
                        menu_ui.mode = MainMenuMode::Hidden;
                        menu_ui.screen = MainMenuScreen::Root;
                        menu_ui.confirm_state = None;
                        menu_ui.post_save_action = None;
                        main_menu.open = false;
                        menu_ui.status_text.clear();
                    }
                    Err(err) => {
                        menu_ui.status_text = format!("New game failed: {}", err);
                    }
                }
            }
            MainMenuButtonAction::OpenSaveScreen => {
                if !world_load_state.has_world {
                    menu_ui.status_text =
                        "No world loaded. Start or load a world before saving.".to_string();
                    continue;
                }
                menu_ui.screen = MainMenuScreen::Save;
                refresh_saves_cache(&mut menu_ui);
            }
            MainMenuButtonAction::OpenLoadScreen => {
                menu_ui.screen = MainMenuScreen::Load;
                refresh_saves_cache(&mut menu_ui);
            }
            MainMenuButtonAction::ExitToMainMenu => {
                if menu_ui.mode != MainMenuMode::InGame {
                    continue;
                }
                if save_session.has_unsaved_changes(step.0) {
                    menu_ui.return_screen = MainMenuScreen::Root;
                    menu_ui.confirm_state = Some(MainMenuConfirmState::UnsavedChanges(
                        MainMenuDeferredAction::ExitToMainMenu,
                    ));
                    menu_ui.confirm_text = "Save changes before exiting to main menu?".to_string();
                    menu_ui.screen = MainMenuScreen::Confirm;
                } else {
                    let state = new_game_snapshot(&gas_registry);
                    match apply_runtime_world_state(
                        state,
                        &mut world,
                        &mut structures,
                        &mut gas,
                        &mut step,
                        &mut world_changed,
                    ) {
                        Ok(_) => {
                            control.paused = true;
                            save_session.mark_persisted(step.0, None);
                            world_load_state.has_world = false;
                            menu_ui.mode = MainMenuMode::Main;
                            menu_ui.screen = MainMenuScreen::Root;
                            menu_ui.confirm_state = None;
                            menu_ui.post_save_action = None;
                            main_menu.open = true;
                            menu_ui.status_text.clear();
                        }
                        Err(err) => {
                            menu_ui.status_text = format!("Exit to main failed: {}", err);
                        }
                    }
                }
            }
            MainMenuButtonAction::ExitApp => {
                if world_load_state.has_world && save_session.has_unsaved_changes(step.0) {
                    menu_ui.return_screen = MainMenuScreen::Root;
                    menu_ui.confirm_state = Some(MainMenuConfirmState::UnsavedChanges(
                        MainMenuDeferredAction::ExitApp,
                    ));
                    menu_ui.confirm_text = "Save changes before exiting the game?".to_string();
                    menu_ui.screen = MainMenuScreen::Confirm;
                } else {
                    exit_writer.write(AppExit::Success);
                }
            }
            MainMenuButtonAction::BackToRoot => {
                menu_ui.screen = MainMenuScreen::Root;
                menu_ui.confirm_state = None;
                menu_ui.confirm_text.clear();
                menu_ui.post_save_action = None;
                menu_ui.status_text.clear();
            }
            MainMenuButtonAction::CreateNewSave => {
                if !world_load_state.has_world {
                    menu_ui.status_text =
                        "No world loaded. Start or load a world before saving.".to_string();
                    continue;
                }
                let name = save_name_input.text.clone();
                match create_save(
                    &saves_root_default(),
                    &name,
                    &world,
                    &gas,
                    &structures,
                    &gas_registry,
                    step.0,
                ) {
                    Ok(descriptor) => {
                        save_session.mark_persisted(step.0, Some(descriptor.id.clone()));
                        refresh_saves_cache(&mut menu_ui);
                        if let Some(post_action) = menu_ui.post_save_action.take() {
                            match post_action {
                                MainMenuDeferredAction::ExitToMainMenu => {
                                    let state = new_game_snapshot(&gas_registry);
                                    match apply_runtime_world_state(
                                        state,
                                        &mut world,
                                        &mut structures,
                                        &mut gas,
                                        &mut step,
                                        &mut world_changed,
                                    ) {
                                        Ok(_) => {
                                            control.paused = true;
                                            save_session.mark_persisted(step.0, None);
                                            world_load_state.has_world = false;
                                            menu_ui.mode = MainMenuMode::Main;
                                            menu_ui.screen = MainMenuScreen::Root;
                                            main_menu.open = true;
                                            menu_ui.status_text.clear();
                                        }
                                        Err(err) => {
                                            menu_ui.status_text =
                                                format!("Exit to main failed: {}", err);
                                            menu_ui.screen = MainMenuScreen::Save;
                                        }
                                    }
                                }
                                MainMenuDeferredAction::ExitApp => {
                                    exit_writer.write(AppExit::Success);
                                }
                            }
                        } else {
                            menu_ui.status_text = format!("Saved '{}'.", descriptor.display_name);
                        }
                    }
                    Err(err) => {
                        menu_ui.status_text = format!("Create save failed: {}", err);
                    }
                }
            }
            MainMenuButtonAction::SelectOverwrite(save_id) => {
                if !world_load_state.has_world {
                    menu_ui.status_text =
                        "No world loaded. Start or load a world before saving.".to_string();
                    continue;
                }
                menu_ui.return_screen = MainMenuScreen::Save;
                menu_ui.confirm_state = Some(MainMenuConfirmState::OverwriteSave(save_id.clone()));
                menu_ui.confirm_text =
                    "This save slot will be fully overwritten. Continue?".to_string();
                menu_ui.screen = MainMenuScreen::Confirm;
            }
            MainMenuButtonAction::SelectLoad(save_id) => {
                match load_save(&saves_root_default(), save_id, &gas_registry) {
                    Ok(loaded) => {
                        match apply_runtime_world_state(
                            loaded.state,
                            &mut world,
                            &mut structures,
                            &mut gas,
                            &mut step,
                            &mut world_changed,
                        ) {
                            Ok(_) => {
                                control.paused = true;
                                save_session.mark_persisted(step.0, Some(loaded.descriptor.id));
                                world_load_state.has_world = true;
                                menu_ui.mode = MainMenuMode::Hidden;
                                menu_ui.screen = MainMenuScreen::Root;
                                menu_ui.confirm_state = None;
                                menu_ui.post_save_action = None;
                                main_menu.open = false;
                                menu_ui.status_text.clear();
                            }
                            Err(err) => {
                                menu_ui.status_text = format!("Load apply failed: {}", err);
                                menu_ui.screen = MainMenuScreen::Load;
                            }
                        }
                    }
                    Err(err) => {
                        menu_ui.status_text = format!("Load failed: {}", err);
                        menu_ui.screen = MainMenuScreen::Load;
                    }
                }
            }
            MainMenuButtonAction::ConfirmPrimary => {
                let confirm = menu_ui.confirm_state.clone();
                menu_ui.confirm_state = None;
                menu_ui.confirm_text.clear();
                match confirm {
                    Some(MainMenuConfirmState::OverwriteSave(save_id)) => {
                        match overwrite_save(
                            &saves_root_default(),
                            &save_id,
                            &world,
                            &gas,
                            &structures,
                            &gas_registry,
                            step.0,
                        ) {
                            Ok(descriptor) => {
                                save_session.mark_persisted(step.0, Some(descriptor.id.clone()));
                                refresh_saves_cache(&mut menu_ui);
                                if let Some(post_action) = menu_ui.post_save_action.take() {
                                    match post_action {
                                        MainMenuDeferredAction::ExitToMainMenu => {
                                            let state = new_game_snapshot(&gas_registry);
                                            match apply_runtime_world_state(
                                                state,
                                                &mut world,
                                                &mut structures,
                                                &mut gas,
                                                &mut step,
                                                &mut world_changed,
                                            ) {
                                                Ok(_) => {
                                                    control.paused = true;
                                                    save_session.mark_persisted(step.0, None);
                                                    world_load_state.has_world = false;
                                                    menu_ui.mode = MainMenuMode::Main;
                                                    menu_ui.screen = MainMenuScreen::Root;
                                                    main_menu.open = true;
                                                    menu_ui.status_text.clear();
                                                }
                                                Err(err) => {
                                                    menu_ui.status_text =
                                                        format!("Exit to main failed: {}", err);
                                                    menu_ui.screen = MainMenuScreen::Save;
                                                }
                                            }
                                        }
                                        MainMenuDeferredAction::ExitApp => {
                                            exit_writer.write(AppExit::Success);
                                        }
                                    }
                                } else {
                                    menu_ui.status_text =
                                        format!("Overwritten '{}'.", descriptor.display_name);
                                    menu_ui.screen = MainMenuScreen::Save;
                                }
                            }
                            Err(err) => {
                                menu_ui.status_text = format!("Overwrite failed: {}", err);
                                menu_ui.screen = MainMenuScreen::Save;
                            }
                        }
                    }
                    Some(MainMenuConfirmState::UnsavedChanges(action)) => {
                        menu_ui.post_save_action = Some(action);
                        menu_ui.screen = MainMenuScreen::Save;
                        refresh_saves_cache(&mut menu_ui);
                    }
                    None => {
                        menu_ui.screen = menu_ui.return_screen;
                    }
                }
            }
            MainMenuButtonAction::ConfirmSecondary => {
                let confirm = menu_ui.confirm_state.clone();
                menu_ui.confirm_state = None;
                menu_ui.confirm_text.clear();
                match confirm {
                    Some(MainMenuConfirmState::OverwriteSave(_)) => {
                        menu_ui.screen = menu_ui.return_screen;
                    }
                    Some(MainMenuConfirmState::UnsavedChanges(action)) => match action {
                        MainMenuDeferredAction::ExitToMainMenu => {
                            let state = new_game_snapshot(&gas_registry);
                            match apply_runtime_world_state(
                                state,
                                &mut world,
                                &mut structures,
                                &mut gas,
                                &mut step,
                                &mut world_changed,
                            ) {
                                Ok(_) => {
                                    control.paused = true;
                                    save_session.mark_persisted(step.0, None);
                                    world_load_state.has_world = false;
                                    menu_ui.mode = MainMenuMode::Main;
                                    menu_ui.screen = MainMenuScreen::Root;
                                    menu_ui.post_save_action = None;
                                    main_menu.open = true;
                                    menu_ui.status_text.clear();
                                }
                                Err(err) => {
                                    menu_ui.status_text = format!("Exit to main failed: {}", err);
                                    menu_ui.screen = MainMenuScreen::Root;
                                }
                            }
                        }
                        MainMenuDeferredAction::ExitApp => {
                            exit_writer.write(AppExit::Success);
                        }
                    },
                    None => {
                        menu_ui.screen = menu_ui.return_screen;
                    }
                }
            }
            MainMenuButtonAction::ConfirmCancel => {
                menu_ui.confirm_state = None;
                menu_ui.confirm_text.clear();
                menu_ui.post_save_action = None;
                menu_ui.screen = menu_ui.return_screen;
            }
        }
    }
}

fn refresh_saves_cache(menu_ui: &mut MainMenuUiState) {
    match list_saves(&saves_root_default()) {
        Ok(saves) => {
            menu_ui.saves = saves;
            menu_ui.needs_save_list_refresh = true;
        }
        Err(err) => {
            menu_ui.saves.clear();
            menu_ui.status_text = format!("Failed to read save list: {}", err);
            menu_ui.needs_save_list_refresh = true;
        }
    }
}

fn apply_runtime_world_state(
    state: crate::save::RuntimeWorldState,
    world: &mut WorldGrid,
    structures: &mut GasStructureGrid,
    gas: &mut GasField,
    step: &mut SimulationStep,
    world_changed: &mut EventWriter<WorldCellChanged>,
) -> Result<(), String> {
    world.restore_from_cell_codes(&state.world_cell_codes)?;
    structures.restore_state(&state.gas_structures_snapshot, world)?;
    gas.restore_state(&state.gas_snapshot)?;
    step.0 = state.simulation_step;
    emit_full_world_changed(world_changed);
    Ok(())
}

fn emit_full_world_changed(world_changed: &mut EventWriter<WorldCellChanged>) {
    for y in 0..WORLD_HEIGHT {
        for x in 0..WORLD_WIDTH {
            world_changed.write(WorldCellChanged {
                cell: UVec2::new(x, y),
            });
        }
    }
}

