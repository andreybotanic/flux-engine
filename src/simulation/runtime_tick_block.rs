fn effective_target_hz(base_hz: u32, speed: SimulationSpeed) -> f64 {
    let base = base_hz.clamp(1, 1000) as f64;
    base * speed.multiplier() as f64
}

fn initialize_gas_field_from_registry(mut commands: Commands, registry: Res<GasRegistry>) {
    commands.insert_resource(GasField::from_registry(&registry));
}

fn apply_fixed_rate_config(
    config: Res<SimulationRateConfig>,
    control: Res<SimulationControl>,
    mut fixed_time: ResMut<Time<Fixed>>,
) {
    if !config.is_changed() && !control.is_changed() {
        return;
    }
    let hz = effective_target_hz(config.target_hz, control.speed);
    fixed_time.set_timestep_hz(hz);
}

fn mark_gpu_state_dirty(
    backend: Res<SimulationBackendConfig>,
    world: Res<WorldGrid>,
    mut gpu_state: ResMut<GpuRuntimeState>,
) {
    if backend.backend != SimulationBackend::Gpu {
        return;
    }
    if world.is_changed() {
        gpu_state.needs_full_upload = true;
    }
}

fn mark_gpu_state_dirty_from_gas_edits(
    backend: Res<SimulationBackendConfig>,
    control: Res<SimulationControl>,
    gas: Res<GasField>,
    mut gpu_state: ResMut<GpuRuntimeState>,
) {
    if backend.backend != SimulationBackend::Gpu {
        return;
    }
    if control.paused && gas.is_changed() {
        gpu_state.needs_full_upload = true;
    }
}

fn run_simulation_tick(
    mut control: ResMut<SimulationControl>,
    backend: Res<SimulationBackendConfig>,
    world_load_state: Res<WorldLoadState>,
    rate: Res<SimulationRateConfig>,
    config: Res<GasSimulationConfig>,
    mut block_state: ResMut<BlockSyncState>,
    mut gas: ResMut<GasField>,
    structures: Res<GasStructureGrid>,
    world: Res<WorldGrid>,
    mut step: ResMut<SimulationStep>,
    mut perf: ResMut<SimulationPerfStats>,
    mut gpu_state: ResMut<GpuRuntimeState>,
) {
    perf.target_hz_effective = effective_target_hz(rate.target_hz, control.speed) as f32;

    if !world_load_state.has_world {
        control.paused = true;
        return;
    }

    if control.paused {
        return;
    }

    let changed_by_structures = apply_gas_structures_pre_step(&structures, &mut gas, &world);
    if changed_by_structures && backend.backend == SimulationBackend::Gpu {
        gpu_state.needs_full_upload = true;
    }

    let started_at = Instant::now();
    match backend.backend {
        SimulationBackend::Cpu => {
            do_one_substep(&mut block_state, &mut gas, &world, &config, &mut step);
            perf.last_gpu_compute_ms = 0.0;
            perf.last_upload_to_gpu_ms = 0.0;
            perf.last_readback_from_gpu_ms = 0.0;
            perf.last_step_total_ms = 0.0;
        }
        SimulationBackend::Gpu => {
            let timings = do_one_substep_gpu(&mut gpu_state, &mut gas, &world, &config, &mut step);
            match timings {
                Ok(t) => {
                    perf.last_gpu_compute_ms = t.compute_gpu_ms;
                    perf.last_upload_to_gpu_ms = t.upload_to_gpu_ms;
                    perf.last_readback_from_gpu_ms = t.readback_from_gpu_ms;
                    perf.last_step_total_ms = t.step_total_ms;
                }
                Err(err) => {
                    abort_on_gpu_runtime_error(&err);
                }
            }
        }
    }
    let elapsed_ms = started_at.elapsed().as_secs_f32() * 1000.0;
    perf.last_step_ms = elapsed_ms;
    perf.avg_step_ms = if perf.avg_step_ms <= f32::EPSILON {
        elapsed_ms
    } else {
        perf.avg_step_ms * 0.9 + elapsed_ms * 0.1
    };
    perf.window_steps = perf.window_steps.saturating_add(1);

    let window_elapsed = perf.window_started_at.elapsed();
    if window_elapsed >= Duration::from_millis(500) {
        let seconds = window_elapsed.as_secs_f32().max(1e-6);
        perf.actual_hz = perf.window_steps as f32 / seconds;
        perf.window_steps = 0;
        perf.window_started_at = Instant::now();
    }
}

pub(crate) fn apply_gas_structures_pre_step(
    structures: &GasStructureGrid,
    gas: &mut GasField,
    world: &WorldGrid,
) -> bool {
    let mut changed = false;
    for (x, y, structure) in structures.iter_cells() {
        match structure {
            GasStructureCell::Source { gas_index, amount } => {
                if gas.add_particles_no_impulse(x, y, gas_index, amount, world) > 0 {
                    changed = true;
                }
            }
            GasStructureCell::Sink { amount } => {
                if gas.remove_particles_proportional(x, y, amount, world) > 0 {
                    changed = true;
                }
            }
        }
    }

    if changed {
        gas.recompute_total_density_buffer(world);
    }

    changed
}

fn do_one_substep_gpu(
    gpu_state: &mut GpuRuntimeState,
    gas: &mut GasField,
    world: &WorldGrid,
    config: &GasSimulationConfig,
    step: &mut SimulationStep,
) -> Result<GpuStepTimings, String> {
    let mut upload_ms = 0.0f32;
    if gpu_state.solver.is_none() {
        let (solver, first_upload_ms) = GpuGasSolver::from_cpu_state(world, gas)?;
        upload_ms += first_upload_ms;
        gpu_state.solver = Some(solver);
        gpu_state.needs_full_upload = false;
        gpu_state.steps_since_readback = 0;
    }

    let solver = gpu_state
        .solver
        .as_mut()
        .ok_or_else(|| "GPU solver missing after initialization".to_string())?;
    if gpu_state.needs_full_upload {
        upload_ms += solver.upload_state(&gas.to_gpu_host_state(world))?;
        gpu_state.needs_full_upload = false;
        gpu_state.steps_since_readback = 0;
    }

    let (width, height) = solver.current_dimensions();
    let params = GpuGasSolver::params_from_config(config, width, height, step.0, gas);
    let mut timings = solver.step(params)?;
    timings.upload_to_gpu_ms += upload_ms;

    gpu_state.steps_since_readback = gpu_state.steps_since_readback.saturating_add(1);
    if gpu_state.steps_since_readback >= GPU_RUNTIME_READBACK_INTERVAL {
        let readback_started = Instant::now();
        let host_state = solver.readback_state()?;
        gas.apply_gpu_host_state(&host_state);
        let readback_ms = readback_started.elapsed().as_secs_f32() * 1000.0;
        timings.readback_from_gpu_ms += readback_ms;
        gpu_state.steps_since_readback = 0;
    }

    step.0 = step.0.saturating_add(1);
    timings.step_total_ms += timings.upload_to_gpu_ms + timings.readback_from_gpu_ms;
    Ok(timings)
}

fn abort_on_gpu_runtime_error(err: &str) -> ! {
    bevy::log::error!(
        "GPU simulation backend failed during runtime. Backend is fixed after startup, aborting process. Details: {}",
        err
    );
    panic!("GPU simulation backend failed: {err}");
}

/// Runs `do_one_substep` logic.
pub fn do_one_substep(
    _block_state: &mut BlockSyncState,
    gas: &mut GasField,
    world: &WorldGrid,
    config: &GasSimulationConfig,
    step: &mut SimulationStep,
) {
    gas.step_discrete(
        world,
        &config.solver_tuning,
        config.thermal_motion_scale,
        step.0,
    );
    step.0 += 1;
}

