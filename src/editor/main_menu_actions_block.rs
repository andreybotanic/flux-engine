fn handle_main_menu_actions(
    mut action_requests: EventReader<MainMenuActionRequest>,
    ui_state: (
        ResMut<MainMenuState>,
        ResMut<MainMenuUiState>,
        ResMut<SimulationControl>,
        Res<GasRegistry>,
        ResMut<SavePreviewQueueState>,
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
    mut camera_query: Query<(&mut Transform, &mut Projection), With<MainCamera>>,
) {
    let pending_actions = action_requests
        .read()
        .map(|event| event.0.clone())
        .collect::<Vec<_>>();
    if pending_actions.is_empty() {
        return;
    }

    let (mut main_menu, mut menu_ui, mut control, gas_registry, mut preview_queue) = ui_state;
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
    if preview_queue.is_busy() {
        menu_ui.status_text = "Please wait until the save preview is ready.".to_string();
        return;
    }

    let saves_root = saves_root_default();
    for action in pending_actions {
        match action {
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
                        let Ok((mut camera_transform, mut camera_projection)) =
                            camera_query.single_mut()
                        else {
                            menu_ui.status_text = "New game failed: main camera missing.".to_string();
                            continue;
                        };
                        apply_loaded_world_preset(
                            &mut control,
                            &mut overlay_mode,
                            &mut camera_transform,
                            &mut camera_projection,
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
                } else if let Err(err) = complete_exit_to_main_menu(
                    &gas_registry,
                    &mut control,
                    &mut main_menu,
                    &mut menu_ui,
                    &mut world,
                    &mut structures,
                    &mut gas,
                    &mut pipe_gas,
                    &mut pipe_flux,
                    &mut step,
                    &mut save_session,
                    &mut world_load_state,
                    &mut world_changed,
                ) {
                    menu_ui.status_text = format!("Exit to main failed: {}", err);
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
                    &saves_root,
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
                        if let Err(err) = queue_save_preview_capture(
                            &saves_root,
                            &mut menu_ui,
                            &mut preview_queue,
                            descriptor.clone(),
                            format!("Saved '{}'.", descriptor.display_name),
                        ) {
                            menu_ui.status_text = format!(
                                "Saved '{}', but preview setup failed: {}",
                                descriptor.display_name, err
                            );
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
                menu_ui.confirm_state = Some(MainMenuConfirmState::OverwriteSave(save_id));
                menu_ui.confirm_text =
                    "This save slot will be fully overwritten. Continue?".to_string();
                menu_ui.screen = MainMenuScreen::Confirm;
            }
            MainMenuButtonAction::SelectDelete(save_id) => {
                open_delete_confirmation(&mut menu_ui, save_id);
            }
            MainMenuButtonAction::SelectLoad(save_id) => {
                match load_save(&saves_root, &save_id, &gas_registry) {
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
                                let Ok((mut camera_transform, mut camera_projection)) =
                                    camera_query.single_mut()
                                else {
                                    menu_ui.status_text =
                                        "Load apply failed: main camera missing.".to_string();
                                    continue;
                                };
                                apply_loaded_world_preset(
                                    &mut control,
                                    &mut overlay_mode,
                                    &mut camera_transform,
                                    &mut camera_projection,
                                );
                                save_session
                                    .mark_persisted(step.0, Some(loaded.descriptor.id.clone()));
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
                            &saves_root,
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
                                if let Err(err) = queue_save_preview_capture(
                                    &saves_root,
                                    &mut menu_ui,
                                    &mut preview_queue,
                                    descriptor.clone(),
                                    format!("Overwritten '{}'.", descriptor.display_name),
                                ) {
                                    menu_ui.status_text = format!(
                                        "Overwritten '{}', but preview setup failed: {}",
                                        descriptor.display_name, err
                                    );
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
                        match delete_save(&saves_root, &save_id) {
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
                            if let Err(err) = complete_exit_to_main_menu(
                                &gas_registry,
                                &mut control,
                                &mut main_menu,
                                &mut menu_ui,
                                &mut world,
                                &mut structures,
                                &mut gas,
                                &mut pipe_gas,
                                &mut pipe_flux,
                                &mut step,
                                &mut save_session,
                                &mut world_load_state,
                                &mut world_changed,
                            ) {
                                menu_ui.status_text = format!("Exit to main failed: {}", err);
                                menu_ui.screen = MainMenuScreen::Root;
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

fn handle_save_preview_capture_finished(
    mut finished: EventReader<SavePreviewCaptureFinished>,
    mut main_menu: ResMut<MainMenuState>,
    mut menu_ui: ResMut<MainMenuUiState>,
    mut control: ResMut<SimulationControl>,
    gas_registry: Res<GasRegistry>,
    mut world: ResMut<WorldGrid>,
    mut structures: ResMut<PlacedStructureMap>,
    mut gas: ResMut<GasField>,
    mut pipe_gas: ResMut<crate::simulation::pipes::PipeGasField>,
    mut pipe_flux: ResMut<crate::simulation::pipes::PipeFluxField>,
    mut step: ResMut<SimulationStep>,
    mut save_session: ResMut<SaveSessionState>,
    mut world_load_state: ResMut<WorldLoadState>,
    mut world_changed: EventWriter<WorldCellChanged>,
    mut exit_writer: EventWriter<AppExit>,
) {
    if finished.is_empty() {
        return;
    }

    for event in finished.read() {
        refresh_saves_cache(&mut menu_ui);
        match &event.result {
            Ok(()) => {
                if let Some(post_action) = event.request.post_save_action.clone() {
                    match post_action {
                        MainMenuDeferredAction::ExitToMainMenu => {
                            if let Err(err) = complete_exit_to_main_menu(
                                &gas_registry,
                                &mut control,
                                &mut main_menu,
                                &mut menu_ui,
                                &mut world,
                                &mut structures,
                                &mut gas,
                                &mut pipe_gas,
                                &mut pipe_flux,
                                &mut step,
                                &mut save_session,
                                &mut world_load_state,
                                &mut world_changed,
                            ) {
                                menu_ui.status_text = format!("Exit to main failed: {}", err);
                                menu_ui.screen = MainMenuScreen::Save;
                            }
                        }
                        MainMenuDeferredAction::ExitApp => {
                            exit_writer.write(AppExit::Success);
                        }
                    }
                } else {
                    menu_ui.status_text = event.request.success_status_text.clone();
                    menu_ui.screen = MainMenuScreen::Save;
                }
            }
            Err(err) => {
                menu_ui.post_save_action = None;
                menu_ui.status_text = format!(
                    "{} Preview capture failed: {}",
                    event.request.success_status_text, err
                );
                menu_ui.screen = MainMenuScreen::Save;
                main_menu.open = true;
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
    restore_runtime_world_state(state, world, structures, gas, pipe_gas, pipe_flux, step)?;
    emit_full_world_changed(world_changed);
    Ok(())
}

fn complete_exit_to_main_menu(
    gas_registry: &GasRegistry,
    control: &mut SimulationControl,
    main_menu: &mut MainMenuState,
    menu_ui: &mut MainMenuUiState,
    world: &mut WorldGrid,
    structures: &mut PlacedStructureMap,
    gas: &mut GasField,
    pipe_gas: &mut crate::simulation::pipes::PipeGasField,
    pipe_flux: &mut crate::simulation::pipes::PipeFluxField,
    step: &mut SimulationStep,
    save_session: &mut SaveSessionState,
    world_load_state: &mut WorldLoadState,
    world_changed: &mut EventWriter<WorldCellChanged>,
) -> Result<(), String> {
    let state = new_game_snapshot(gas_registry);
    apply_runtime_world_state(
        state,
        world,
        structures,
        gas,
        pipe_gas,
        pipe_flux,
        step,
        world_changed,
    )?;
    control.paused = true;
    save_session.mark_persisted(step.0, None);
    world_load_state.has_world = false;
    menu_ui.mode = MainMenuMode::Main;
    menu_ui.screen = MainMenuScreen::Root;
    menu_ui.confirm_state = None;
    menu_ui.confirm_text.clear();
    menu_ui.post_save_action = None;
    main_menu.open = true;
    menu_ui.status_text.clear();
    Ok(())
}

fn queue_save_preview_capture(
    root: &std::path::Path,
    menu_ui: &mut MainMenuUiState,
    preview_queue: &mut SavePreviewQueueState,
    descriptor: crate::save::SaveDescriptor,
    success_status_text: String,
) -> Result<(), String> {
    if preview_queue.is_busy() {
        return Err("another save preview capture is already running".to_string());
    }
    let post_save_action = menu_ui.post_save_action.take();
    let target_path = descriptor
        .preview_path
        .clone()
        .unwrap_or_else(|| save_preview_target_path(root, &descriptor.id));
    preview_queue.pending = Some(SavePreviewRequest {
        descriptor,
        target_path,
        post_save_action,
        success_status_text,
    });
    menu_ui.confirm_state = None;
    menu_ui.confirm_text.clear();
    menu_ui.screen = MainMenuScreen::Save;
    menu_ui.status_text = "Generating save preview...".to_string();
    Ok(())
}

fn open_delete_confirmation(menu_ui: &mut MainMenuUiState, save_id: String) {
    menu_ui.return_screen = menu_ui.screen;
    menu_ui.confirm_state = Some(MainMenuConfirmState::DeleteSave(save_id));
    menu_ui.confirm_text = "This save slot will be deleted permanently. Continue?".to_string();
    menu_ui.screen = MainMenuScreen::Confirm;
}

#[cfg(test)]
mod main_menu_actions_tests {
    use super::queue_save_preview_capture;
    use crate::{
        config::{GasDefinition, GasRegistry},
        save::{
            new_game_snapshot, restore_runtime_world_state, MainMenuDeferredAction, MainMenuUiState,
            SaveDescriptor, SavePreviewQueueState,
        },
        simulation::{
            gas::GasField,
            pipes::{apply_pipe_network_step, PipeFlowVisualState, PipeFluxField, PipeGasField},
            PipeSimulationConfig, SimulationStep,
        },
        world::{grid::WorldGrid, structures::PlacedStructureMap},
    };
    use std::path::Path;

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

        let state = new_game_snapshot(&registry);
        restore_runtime_world_state(
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
        assert_eq!(step.0, 0);
    }

    #[test]
    fn queueing_preview_keeps_post_save_action_pending_until_capture_finishes() {
        let mut menu_ui = MainMenuUiState::default();
        let mut preview_queue = SavePreviewQueueState::default();
        menu_ui.post_save_action = Some(MainMenuDeferredAction::ExitApp);
        let descriptor = SaveDescriptor {
            id: "slot-1".to_string(),
            display_name: "Slot 1".to_string(),
            created_at_unix_ms: 0,
            updated_at_unix_ms: 0,
            preview_path: None,
        };

        queue_save_preview_capture(
            Path::new("D:/tmp"),
            &mut menu_ui,
            &mut preview_queue,
            descriptor,
            "Saved 'Slot 1'.".to_string(),
        )
        .expect("queue preview");

        assert!(menu_ui.post_save_action.is_none());
        let request = preview_queue.pending.expect("queued request");
        assert!(matches!(
            request.post_save_action,
            Some(MainMenuDeferredAction::ExitApp)
        ));
    }
}
