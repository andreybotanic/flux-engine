fn handle_main_menu_actions(
    mut interactions: Query<
        (&Interaction, &MainMenuActionButton, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    ui_state: (
        ResMut<MainMenuState>,
        ResMut<MainMenuUiState>,
        ResMut<SimulationControl>,
        Res<GasRegistry>,
    ),
    world_state: (
        ResMut<WorldGrid>,
        ResMut<PlacedStructureMap>,
        ResMut<GasField>,
        ResMut<crate::simulation::pipes::PipeGasField>,
        ResMut<crate::simulation::pipes::PipeFluxField>,
        ResMut<SimulationStep>,
        ResMut<SaveSessionState>,
        ResMut<WorldLoadState>,
    ),
    save_name_input: Single<&TextInputField, With<MainMenuSaveNameInputField>>,
    mut world_changed: EventWriter<WorldCellChanged>,
    mut exit_writer: EventWriter<AppExit>,
    mut overlay_mode: ResMut<OverlayMode>,
    mut camera_query: Single<(&mut Transform, &mut Projection), With<MainCamera>>,
) {
    let (mut main_menu, mut menu_ui, mut control, gas_registry) = ui_state;
    let (
        mut world,
        mut structures,
        mut gas,
        mut pipe_gas,
        mut pipe_flux,
        mut step,
        mut save_session,
        mut world_load_state,
    ) = world_state;
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
                    &mut pipe_gas,
                    &mut pipe_flux,
                    &mut step,
                    &mut world_changed,
                ) {
                    Ok(_) => {
                        apply_loaded_world_preset(
                            &mut control,
                            &mut overlay_mode,
                            &mut camera_query,
                        );
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
                        &mut pipe_gas,
                        &mut pipe_flux,
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
                    &pipe_gas,
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
                                        &mut pipe_gas,
                                        &mut pipe_flux,
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
            MainMenuButtonAction::SelectDelete(save_id) => {
                open_delete_confirmation(&mut menu_ui, save_id.clone());
            }
            MainMenuButtonAction::SelectLoad(save_id) => {
                match load_save(&saves_root_default(), save_id, &gas_registry) {
                    Ok(loaded) => {
                        match apply_runtime_world_state(
                            loaded.state,
                            &mut world,
                            &mut structures,
                            &mut gas,
                            &mut pipe_gas,
                            &mut pipe_flux,
                            &mut step,
                            &mut world_changed,
                        ) {
                            Ok(_) => {
                                apply_loaded_world_preset(
                                    &mut control,
                                    &mut overlay_mode,
                                    &mut camera_query,
                                );
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
                            &pipe_gas,
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
                                                &mut pipe_gas,
                                                &mut pipe_flux,
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
                    Some(MainMenuConfirmState::DeleteSave(save_id)) => {
                        match delete_save(&saves_root_default(), &save_id) {
                            Ok(()) => {
                                if save_session.current_save_id.as_deref() == Some(save_id.as_str()) {
                                    save_session.current_save_id = None;
                                }
                                refresh_saves_cache(&mut menu_ui);
                                menu_ui.status_text = "Save deleted.".to_string();
                                menu_ui.screen = menu_ui.return_screen;
                            }
                            Err(err) => {
                                menu_ui.status_text = format!("Delete failed: {}", err);
                                menu_ui.screen = menu_ui.return_screen;
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
                    Some(MainMenuConfirmState::OverwriteSave(_))
                    | Some(MainMenuConfirmState::DeleteSave(_)) => {
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
                            &mut pipe_gas,
                            &mut pipe_flux,
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
    structures: &mut PlacedStructureMap,
    gas: &mut GasField,
    pipe_gas: &mut crate::simulation::pipes::PipeGasField,
    pipe_flux: &mut crate::simulation::pipes::PipeFluxField,
    step: &mut SimulationStep,
    world_changed: &mut EventWriter<WorldCellChanged>,
) -> Result<(), String> {
    restore_runtime_world_resources(state, world, structures, gas, pipe_gas, pipe_flux, step)?;
    emit_full_world_changed(world_changed);
    Ok(())
}

fn restore_runtime_world_resources(
    state: crate::save::RuntimeWorldState,
    world: &mut WorldGrid,
    structures: &mut PlacedStructureMap,
    gas: &mut GasField,
    pipe_gas: &mut crate::simulation::pipes::PipeGasField,
    pipe_flux: &mut crate::simulation::pipes::PipeFluxField,
    step: &mut SimulationStep,
) -> Result<(), String> {
    world.restore_from_cell_codes(&state.world_cell_codes)?;
    structures.restore_state(&state.placed_structures_snapshot, world)?;
    gas.restore_state(&state.gas_snapshot)?;
    pipe_gas.restore_state(&state.pipe_gas_snapshot, structures)?;
    pipe_flux.clear_all();
    step.0 = state.simulation_step;
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

fn apply_loaded_world_preset(
    control: &mut SimulationControl,
    overlay_mode: &mut OverlayMode,
    camera_query: &mut Single<(&mut Transform, &mut Projection), With<MainCamera>>,
) {
    control.paused = true;
    control.speed = crate::simulation::SimulationSpeed::X1;
    *overlay_mode = OverlayMode::Main;
    let (transform, projection) = &mut **camera_query;
    crate::input::camera::reset_camera_to_default(transform, projection);
}

fn open_delete_confirmation(menu_ui: &mut MainMenuUiState, save_id: String) {
    menu_ui.return_screen = menu_ui.screen;
    menu_ui.confirm_state = Some(MainMenuConfirmState::DeleteSave(save_id));
    menu_ui.confirm_text = "This save slot will be deleted permanently. Continue?".to_string();
    menu_ui.screen = MainMenuScreen::Confirm;
}

#[cfg(test)]
mod main_menu_actions_tests {
    use super::restore_runtime_world_resources;
    use crate::{
        config::{GasDefinition, GasRegistry},
        save::new_game_snapshot,
        simulation::{
            gas::GasField,
            pipes::{apply_pipe_network_step, PipeFlowVisualState, PipeFluxField, PipeGasField},
            PipeSimulationConfig, SimulationStep,
        },
        world::{grid::WorldGrid, structures::PlacedStructureMap},
    };

    fn registry() -> GasRegistry {
        GasRegistry::new(vec![GasDefinition {
            id: "h2".to_string(),
            label: "Hydrogen".to_string(),
            molecular_mass: 2.016,
            color: [0.7, 0.8, 1.0],
        }])
        .expect("test registry")
    }

    #[test]
    fn restoring_runtime_world_resets_pipe_flux_but_keeps_saved_pipe_gas() {
        let registry = registry();
        let mut world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        let mut gas = GasField::from_registry(&registry);
        let mut pipe_gas = PipeGasField::from_registry(&registry);
        let mut pipe_flux = PipeFluxField::default();
        let mut step = SimulationStep(99);

        assert!(structures.place_pipe(8, 8, &world));
        assert!(structures.place_pipe(9, 8, &world));
        assert!(structures.place_vent(8, 8, &world));
        pipe_gas.sync_to_structures(&structures);
        gas.set_amount(8, 8, 0, 100_000.0);
        let _ = apply_pipe_network_step(
            &structures,
            &mut pipe_gas,
            &mut pipe_flux,
            &mut gas,
            &world,
            &mut PipeFlowVisualState::default(),
            &PipeSimulationConfig::default(),
        );
        assert!(pipe_flux.edge_count() > 0);
        assert!(pipe_flux.max_abs_flux() > 0.0);

        let state = new_game_snapshot(&registry);
        restore_runtime_world_resources(
            state,
            &mut world,
            &mut structures,
            &mut gas,
            &mut pipe_gas,
            &mut pipe_flux,
            &mut step,
        )
        .expect("restore runtime world");

        assert_eq!(pipe_gas.node_count(), 0);
        assert_eq!(pipe_flux.edge_count(), 0);
        assert_eq!(pipe_flux.max_abs_flux(), 0.0);
        assert_eq!(step.0, 0);
    }
}
