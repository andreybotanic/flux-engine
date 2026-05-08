use std::{collections::VecDeque, path::PathBuf};

use bevy::{
    app::AppExit,
    asset::AssetPlugin,
    prelude::*,
    window::{PresentMode, WindowPlugin, WindowResolution},
};
use flux_engine::{
    config::GameConfig,
    editor::StructureEditState,
    input::InputPlugin,
    render::RenderPlugin,
    save::{
        emit_full_world_changed, list_saves, load_save, save_preview_target_path,
        saves_root_default, restore_runtime_world_state, SaveDescriptor, WorldLoadState,
        SavePreviewCaptureFinished, SavePreviewQueueState, SavePreviewRequest,
    },
    simulation::{
        backend::{SimulationBackend, SimulationBackendConfig, WorldSizeConfig},
        gas::GasField,
        pipes::{PipeFluxField, PipeGasField},
        GasSimulationPlugin, SimulationStep,
    },
    world::{grid::WorldGrid, structures::PlacedStructureMap, WorldCellChanged, WorldPlugin},
};

#[derive(Resource)]
/// Tracks the one-shot save-preview migration job queue and summary.
struct SavePreviewMigrationState {
    root: PathBuf,
    pending: VecDeque<SaveDescriptor>,
    active_save_id: Option<String>,
    migrated: usize,
    skipped: usize,
    failed: Vec<String>,
    finished: bool,
}

impl SavePreviewMigrationState {
    fn new(root: PathBuf) -> Self {
        let mut pending = VecDeque::new();
        let mut skipped = 0usize;

        match list_saves(&root) {
            Ok(saves) => {
                for descriptor in saves {
                    let has_valid_preview = descriptor
                        .preview_path
                        .as_ref()
                        .map(|path| image::image_dimensions(path).is_ok())
                        .unwrap_or(false);
                    if has_valid_preview {
                        skipped += 1;
                    } else {
                        pending.push_back(descriptor);
                    }
                }
            }
            Err(err) => {
                pending.clear();
                let failed = vec![format!("Failed to read saves list from '{}': {}", root.display(), err)];
                return Self {
                    root,
                    pending,
                    active_save_id: None,
                    migrated: 0,
                    skipped,
                    failed,
                    finished: false,
                };
            }
        }

        Self {
            root,
            pending,
            active_save_id: None,
            migrated: 0,
            skipped,
            failed: Vec::new(),
            finished: false,
        }
    }
}

fn hide_runtime_ui(mut commands: Commands) {
    commands.insert_resource(StructureEditState::default());
}

fn queue_next_preview_migration(
    mut migration: ResMut<SavePreviewMigrationState>,
    gas_registry: Res<flux_engine::config::GasRegistry>,
    mut preview_queue: ResMut<SavePreviewQueueState>,
    mut world: ResMut<WorldGrid>,
    mut structures: ResMut<PlacedStructureMap>,
    mut gas: ResMut<GasField>,
    mut pipe_gas: ResMut<PipeGasField>,
    mut pipe_flux: ResMut<PipeFluxField>,
    mut step: ResMut<SimulationStep>,
    mut world_load_state: ResMut<WorldLoadState>,
    mut world_changed: EventWriter<WorldCellChanged>,
) {
    if migration.finished || migration.active_save_id.is_some() || preview_queue.is_busy() {
        return;
    }
    let Some(descriptor) = migration.pending.pop_front() else {
        return;
    };

    match load_save(&migration.root, &descriptor.id, &gas_registry) {
        Ok(loaded) => {
            if let Err(err) = restore_runtime_world_state(
                loaded.state,
                &mut world,
                &mut structures,
                &mut gas,
                &mut pipe_gas,
                &mut pipe_flux,
                &mut step,
            ) {
                migration.failed.push(format!(
                    "{} ({}) restore failed: {}",
                    descriptor.id, descriptor.display_name, err
                ));
                return;
            }
            emit_full_world_changed(&mut world_changed);
            world_load_state.has_world = true;
            let target_path = save_preview_target_path(&migration.root, &descriptor.id);
            preview_queue.pending = Some(SavePreviewRequest {
                root: migration.root.clone(),
                descriptor: loaded.descriptor.clone(),
                target_path,
                post_save_action: None,
                success_status_text: format!("Migrated '{}'.", loaded.descriptor.display_name),
                patch_meta_on_success: true,
            });
            migration.active_save_id = Some(descriptor.id.clone());
            println!("Migrating preview for '{}' ({})", descriptor.display_name, descriptor.id);
        }
        Err(err) => {
            migration.failed.push(format!(
                "{} ({}) load failed: {}",
                descriptor.id, descriptor.display_name, err
            ));
        }
    }
}

fn collect_preview_migration_results(
    mut migration: ResMut<SavePreviewMigrationState>,
    mut finished: EventReader<SavePreviewCaptureFinished>,
) {
    for event in finished.read() {
        migration.active_save_id = None;
        match &event.result {
            Ok(()) => {
                migration.migrated += 1;
                println!(
                    "Migrated preview for '{}' ({})",
                    event.request.descriptor.display_name, event.request.descriptor.id
                );
            }
            Err(err) => {
                migration.failed.push(format!(
                    "{} ({}) preview failed: {}",
                    event.request.descriptor.id, event.request.descriptor.display_name, err
                ));
            }
        }
    }
}

fn finish_preview_migration(
    mut migration: ResMut<SavePreviewMigrationState>,
    preview_queue: Res<SavePreviewQueueState>,
    mut exit_writer: EventWriter<AppExit>,
) {
    if migration.finished || migration.active_save_id.is_some() || preview_queue.is_busy() {
        return;
    }
    if !migration.pending.is_empty() {
        return;
    }

    migration.finished = true;
    println!(
        "Preview migration summary: migrated={}, skipped={}, failed={}",
        migration.migrated,
        migration.skipped,
        migration.failed.len()
    );
    for failure in &migration.failed {
        eprintln!("{failure}");
    }

    if migration.failed.is_empty() {
        exit_writer.write(AppExit::Success);
    } else {
        exit_writer.write(AppExit::error());
    }
}

fn main() -> AppExit {
    let game_config = GameConfig::load_from_default_location().unwrap_or_else(|err| {
        panic!("Failed to load game config files from ./config: {err}");
    });
    let asset_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .to_string_lossy()
        .to_string();
    let saves_root = saves_root_default();

    let mut app = App::new();
    app.insert_resource(ClearColor(Color::BLACK))
        .insert_resource(Time::<Fixed>::from_hz(30.0))
        .insert_resource(game_config.gas_registry.clone())
        .insert_resource(game_config.simulation_rate)
        .insert_resource(game_config.gas_simulation)
        .insert_resource(game_config.gas_visual)
        .insert_resource(game_config.gas_main_visual)
        .insert_resource(game_config.cell_visuals)
        .insert_resource(game_config.structure_visuals)
        .insert_resource(game_config.cell_visual_layouts)
        .insert_resource(SimulationBackendConfig {
            backend: SimulationBackend::Cpu,
        })
        .insert_resource(WorldSizeConfig::default())
        .insert_resource(WorldLoadState::default())
        .insert_resource(SavePreviewMigrationState::new(saves_root))
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: asset_path,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "FluxEngine Preview Migration".into(),
                        resolution: WindowResolution::new(64.0, 64.0),
                        present_mode: PresentMode::AutoNoVsync,
                        visible: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins((WorldPlugin, GasSimulationPlugin, RenderPlugin, InputPlugin))
        .add_systems(Startup, hide_runtime_ui)
        .add_systems(
            Update,
            (
                queue_next_preview_migration,
                collect_preview_migration_results,
                finish_preview_migration,
            )
                .chain(),
        );

    app.run()
}
