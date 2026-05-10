/// Runs `compare_cpu_gpu_fields` logic.
pub fn compare_cpu_gpu_fields(
    cpu: &GasField,
    gpu: &GasField,
    world: &WorldGrid,
    radius: u32,
) -> ParityMetrics {
    let gas_count = cpu.gas_count().min(gpu.gas_count());
    let mut errors = Vec::new();
    for y in 1..WORLD_HEIGHT - 1 {
        for x in 1..WORLD_WIDTH - 1 {
            if world.is_solid(x, y) {
                continue;
            }
            for gas_index in 0..gas_count {
                let a = local_average(cpu, world, x, y, gas_index, radius);
                let b = local_average(gpu, world, x, y, gas_index, radius);
                if let (Some(a), Some(b)) = (a, b) {
                    errors.push((a - b).abs());
                }
            }
        }
    }

    let mean_abs_error = if errors.is_empty() {
        0.0
    } else {
        errors.iter().sum::<f32>() / errors.len() as f32
    };
    let p95_abs_error = percentile(&errors, 0.95);
    let max_abs_error = errors.iter().copied().fold(0.0f32, f32::max);

    let cpu_totals = cpu.species_totals_u64(world);
    let gpu_totals = gpu.species_totals_u64(world);
    let mut mass_rel_errors = vec![0.0f32; gas_count];
    for gas_index in 0..gas_count {
        let a = cpu_totals[gas_index] as f64;
        let b = gpu_totals[gas_index] as f64;
        let denom = a.max(1.0);
        mass_rel_errors[gas_index] = ((a - b).abs() / denom) as f32;
    }

    ParityMetrics {
        compared_cells: errors.len(),
        mean_abs_error,
        p95_abs_error,
        max_abs_error,
        mass_rel_errors,
    }
}

/// Runs `run_cpu_gpu_parity_scenario` logic.
pub fn run_cpu_gpu_parity_scenario(
    scenario: ScenarioSpec,
    steps: u32,
    radius: u32,
) -> Result<ParityMetrics, String> {
    let mut world = WorldGrid::default();
    let mut structures = PlacedStructureMap::default();
    let registry = test_registry_three_gases();
    let config = tuned_config();

    let mut cpu_field = GasField::from_registry(&registry);
    populate_scenario(
        &mut world,
        &mut structures,
        &mut cpu_field,
        scenario.with_internal_walls,
        scenario.with_structures,
    );

    let mut gpu_field = cpu_field.clone();
    let mut cpu_step = SimulationStep(0);
    let mut gpu_step = SimulationStep(0);
    let mut block_sync = super::BlockSyncState;

    for _ in 0..steps {
        let _ = apply_gas_structures_pre_step(&structures, &mut cpu_field, &world);
        super::do_one_substep(
            &mut block_sync,
            &mut cpu_field,
            &world,
            &config,
            &mut cpu_step,
        );
    }

    let mut solver = GpuGasSolver::from_cpu_state(&world, &gpu_field)?.0;
    for _ in 0..steps {
        let changed = apply_gas_structures_pre_step(&structures, &mut gpu_field, &world);
        if changed {
            let _ = solver.upload_state(&gpu_field.to_gpu_host_state(&world))?;
        }
        let params = GpuGasSolver::params_from_config(
            &config,
            WORLD_WIDTH,
            WORLD_HEIGHT,
            gpu_step.0,
            &gpu_field,
        );
        let _ = solver.step(params)?;
        gpu_step.0 = gpu_step.0.saturating_add(1);
    }
    let host = solver.readback_state()?;
    gpu_field.apply_gpu_host_state(&host);

    Ok(compare_cpu_gpu_fields(
        &cpu_field, &gpu_field, &world, radius,
    ))
}

/// Runs `run_cpu_cpu_parity_scenario` logic.
pub fn run_cpu_cpu_parity_scenario(
    scenario: ScenarioSpec,
    steps: u32,
    radius: u32,
    step_offset_a: u64,
    step_offset_b: u64,
) -> Result<ParityMetrics, String> {
    let mut world = WorldGrid::default();
    let mut structures = PlacedStructureMap::default();
    let registry = test_registry_three_gases();
    let config = tuned_config();

    let mut base_field = GasField::from_registry(&registry);
    populate_scenario(
        &mut world,
        &mut structures,
        &mut base_field,
        scenario.with_internal_walls,
        scenario.with_structures,
    );

    let mut cpu_a = base_field.clone();
    let mut cpu_b = base_field;
    let mut step_a = SimulationStep(step_offset_a);
    let mut step_b = SimulationStep(step_offset_b);
    let mut block_sync_a = super::BlockSyncState;
    let mut block_sync_b = super::BlockSyncState;

    for _ in 0..steps {
        let _ = apply_gas_structures_pre_step(&structures, &mut cpu_a, &world);
        super::do_one_substep(&mut block_sync_a, &mut cpu_a, &world, &config, &mut step_a);
        let _ = apply_gas_structures_pre_step(&structures, &mut cpu_b, &world);
        super::do_one_substep(&mut block_sync_b, &mut cpu_b, &world, &config, &mut step_b);
    }

    Ok(compare_cpu_gpu_fields(&cpu_a, &cpu_b, &world, radius))
}

/// Runs `parity_passes` logic.
pub fn parity_passes(metrics: &ParityMetrics, thresholds: ParityThresholds) -> bool {
    if metrics.mean_abs_error > thresholds.mean_abs_error_max {
        return false;
    }
    if metrics.p95_abs_error > thresholds.p95_abs_error_max {
        return false;
    }
    if metrics.max_abs_error > thresholds.max_abs_error_max {
        return false;
    }
    metrics
        .mass_rel_errors
        .iter()
        .all(|v| *v <= thresholds.mass_rel_error_max)
}

/// Runs `run_full_parity_gate` logic.
pub fn run_full_parity_gate() -> Result<Vec<(ScenarioSpec, ParityMetrics)>, String> {
    let mut out = Vec::new();
    for scenario in PARITY_SCENARIOS {
        let metrics =
            run_cpu_gpu_parity_scenario(scenario, PARITY_STEPS, PARITY_THRESHOLDS.radius)?;
        if !parity_passes(&metrics, PARITY_THRESHOLDS) {
            return Err(format!(
                "Parity gate failed for scenario '{}': mae={:.3}, p95={:.3}, max={:.3}, mass={:?}",
                scenario.name,
                metrics.mean_abs_error,
                metrics.p95_abs_error,
                metrics.max_abs_error,
                metrics.mass_rel_errors
            ));
        }
        out.push((scenario, metrics));
    }
    Ok(out)
}

/// Runs `run_cpu_only_calibration` logic.
pub fn run_cpu_only_calibration(
    step_offsets: &[u64],
    radius: u32,
) -> Result<ParityMetrics, String> {
    if step_offsets.len() < 2 {
        return Err("Calibration requires at least two CPU runs".to_string());
    }
    let mut world = WorldGrid::default();
    let mut structures = PlacedStructureMap::default();
    let registry = test_registry_three_gases();
    let config = tuned_config();

    let mut states = Vec::new();
    for offset in step_offsets {
        let mut field = GasField::from_registry(&registry);
        populate_scenario(&mut world, &mut structures, &mut field, true, false);
        let mut step = SimulationStep(*offset);
        let mut block_sync = super::BlockSyncState;
        for _ in 0..PARITY_STEPS {
            let _ = apply_gas_structures_pre_step(&structures, &mut field, &world);
            super::do_one_substep(&mut block_sync, &mut field, &world, &config, &mut step);
        }
        states.push(field);
    }

    let mut all_errors = Vec::new();
    let mut mass_errors: Vec<f32> = Vec::new();
    for i in 0..states.len() {
        for j in i + 1..states.len() {
            let metrics = compare_cpu_gpu_fields(&states[i], &states[j], &world, radius);
            all_errors.push(metrics.mean_abs_error);
            all_errors.push(metrics.p95_abs_error);
            all_errors.push(metrics.max_abs_error);
            if mass_errors.len() < metrics.mass_rel_errors.len() {
                mass_errors.resize(metrics.mass_rel_errors.len(), 0.0);
            }
            for (k, v) in metrics.mass_rel_errors.iter().enumerate() {
                mass_errors[k] = mass_errors[k].max(*v);
            }
        }
    }

    let mean_abs_error = if all_errors.is_empty() {
        0.0
    } else {
        all_errors.iter().sum::<f32>() / all_errors.len() as f32
    };
    Ok(ParityMetrics {
        compared_cells: all_errors.len(),
        mean_abs_error,
        p95_abs_error: percentile(&all_errors, 0.95),
        max_abs_error: all_errors.iter().copied().fold(0.0f32, f32::max),
        mass_rel_errors: mass_errors,
    })
}
