use crate::{
    config::{GasDefinition, GasRegistry},
    simulation::{
        apply_gas_structures_pre_step, gas::GasField, gpu_solver::GpuGasSolver,
        GasSimulationConfig, SimulationStep, SolverTuning,
    },
    world::{
        gas_structures::GasStructureGrid,
        grid::{is_boundary, CellMaterial, WorldGrid, WORLD_HEIGHT, WORLD_WIDTH},
    },
};
use bevy::prelude::UVec2;

#[derive(Clone, Copy, Debug)]
pub struct ParityThresholds {
    pub radius: u32,
    pub mean_abs_error_max: f32,
    pub p95_abs_error_max: f32,
    pub max_abs_error_max: f32,
    pub mass_rel_error_max: f32,
}

#[derive(Clone, Debug)]
pub struct ParityMetrics {
    pub compared_cells: usize,
    pub mean_abs_error: f32,
    pub p95_abs_error: f32,
    pub max_abs_error: f32,
    pub mass_rel_errors: [f32; 3],
}

#[derive(Clone, Copy, Debug)]
pub struct ScenarioSpec {
    pub name: &'static str,
    pub with_internal_walls: bool,
    pub with_structures: bool,
}

pub const PARITY_STEPS: u32 = 5_000;
pub const PARITY_THRESHOLDS: ParityThresholds = ParityThresholds {
    radius: 3,
    mean_abs_error_max: 150.0,
    p95_abs_error_max: 700.0,
    max_abs_error_max: 2_500.0,
    mass_rel_error_max: 0.50,
};

pub const PARITY_SCENARIOS: [ScenarioSpec; 3] = [
    ScenarioSpec {
        name: "open-field",
        with_internal_walls: false,
        with_structures: false,
    },
    ScenarioSpec {
        name: "inner-walls",
        with_internal_walls: true,
        with_structures: false,
    },
    ScenarioSpec {
        name: "inner-walls-with-structures",
        with_internal_walls: true,
        with_structures: true,
    },
];

fn test_registry_three_gases() -> GasRegistry {
    GasRegistry::new(vec![
        GasDefinition {
            id: "h2".to_string(),
            label: "Hydrogen".to_string(),
            color: [0.65, 0.85, 1.0],
            molecular_mass: 2.016,
        },
        GasDefinition {
            id: "o2".to_string(),
            label: "Oxygen".to_string(),
            color: [0.6, 0.8, 1.0],
            molecular_mass: 31.998,
        },
        GasDefinition {
            id: "co2".to_string(),
            label: "Carbon Dioxide".to_string(),
            color: [0.9, 0.6, 0.4],
            molecular_mass: 44.009,
        },
    ])
    .expect("test gas registry")
}

fn tuned_config() -> GasSimulationConfig {
    GasSimulationConfig {
        thermal_motion_scale: 0.08,
        solver_tuning: SolverTuning::default(),
        ..GasSimulationConfig::default()
    }
}

fn populate_scenario(
    world: &mut WorldGrid,
    structures: &mut GasStructureGrid,
    gas: &mut GasField,
    with_internal_walls: bool,
    with_structures: bool,
) {
    gas.clear_rect(
        UVec2::new(1, 1),
        UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
    );
    *structures = GasStructureGrid::default();
    if with_internal_walls {
        for x in 20..=80 {
            let _ = world.set_solid_with_material(x, 20, CellMaterial::Brick);
            let _ = world.set_solid_with_material(x, 80, CellMaterial::Brick);
        }
        for y in 20..=80 {
            let _ = world.set_solid_with_material(20, y, CellMaterial::Brick);
            let _ = world.set_solid_with_material(80, y, CellMaterial::Brick);
        }
        for x in 48..=53 {
            let _ = world.set_empty(x, 20);
        }
        for y in 34..=71 {
            let _ = world.set_solid_with_material(50, y, CellMaterial::Metal);
        }
        let _ = world.set_empty(50, 52);
        let _ = world.set_empty(50, 53);
    }

    if with_structures {
        let _ = structures.set_source(28, 28, 0, 8, world);
        let _ = structures.set_source(74, 74, 2, 11, world);
        let _ = structures.set_sink(28, 74, 7, world);
        let _ = structures.set_sink(74, 28, 9, world);
    }

    for y in 1..WORLD_HEIGHT - 1 {
        for x in 1..WORLD_WIDTH - 1 {
            if is_boundary(x, y) || world.is_solid(x, y) {
                continue;
            }
            let a0 = ((x * 31 + y * 17) % 43) as f32;
            let a1 = ((x * 7 + y * 19) % 29) as f32;
            let a2 = ((x * 23 + y * 13) % 37) as f32;
            if (x + y) % 2 == 0 {
                gas.set_amount(x, y, 0, a0 + 10.0);
            }
            if (x * 2 + y) % 3 == 0 {
                gas.set_amount(x, y, 1, a1 + 6.0);
            }
            if (x + y * 2) % 5 == 0 {
                gas.set_amount(x, y, 2, a2 + 4.0);
            }
        }
    }
    gas.recompute_total_density_buffer(world);
}

fn percentile(values: &[f32], pct: f32) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((sorted.len() - 1) as f32 * pct).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn local_average(
    field: &GasField,
    world: &WorldGrid,
    x: u32,
    y: u32,
    gas_index: usize,
    radius: u32,
) -> Option<f32> {
    let mut sum = 0.0f32;
    let mut n = 0u32;
    let r2 = i32::try_from(radius.saturating_mul(radius)).unwrap_or(i32::MAX);
    for dy in -(radius as i32)..=(radius as i32) {
        for dx in -(radius as i32)..=(radius as i32) {
            if dx * dx + dy * dy > r2 {
                continue;
            }
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx <= 0
                || ny <= 0
                || nx >= (WORLD_WIDTH - 1) as i32
                || ny >= (WORLD_HEIGHT - 1) as i32
            {
                continue;
            }
            let nx = nx as u32;
            let ny = ny as u32;
            if world.is_solid(nx, ny) {
                continue;
            }
            sum += field.amount(nx, ny, gas_index);
            n += 1;
        }
    }
    if n == 0 {
        None
    } else {
        Some(sum / n as f32)
    }
}

pub fn compare_cpu_gpu_fields(
    cpu: &GasField,
    gpu: &GasField,
    world: &WorldGrid,
    radius: u32,
) -> ParityMetrics {
    let gas_count = cpu.gas_count().min(gpu.gas_count()).min(3);
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
    let mut mass_rel_errors = [0.0f32; 3];
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

pub fn run_cpu_gpu_parity_scenario(
    scenario: ScenarioSpec,
    steps: u32,
    radius: u32,
) -> Result<ParityMetrics, String> {
    let mut world = WorldGrid::default();
    let mut structures = GasStructureGrid::default();
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

pub fn run_cpu_cpu_parity_scenario(
    scenario: ScenarioSpec,
    steps: u32,
    radius: u32,
    step_offset_a: u64,
    step_offset_b: u64,
) -> Result<ParityMetrics, String> {
    let mut world = WorldGrid::default();
    let mut structures = GasStructureGrid::default();
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
        super::do_one_substep(
            &mut block_sync_a,
            &mut cpu_a,
            &world,
            &config,
            &mut step_a,
        );
        let _ = apply_gas_structures_pre_step(&structures, &mut cpu_b, &world);
        super::do_one_substep(
            &mut block_sync_b,
            &mut cpu_b,
            &world,
            &config,
            &mut step_b,
        );
    }

    Ok(compare_cpu_gpu_fields(&cpu_a, &cpu_b, &world, radius))
}

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

pub fn run_cpu_only_calibration(
    step_offsets: &[u64],
    radius: u32,
) -> Result<ParityMetrics, String> {
    if step_offsets.len() < 2 {
        return Err("Calibration requires at least two CPU runs".to_string());
    }
    let mut world = WorldGrid::default();
    let mut structures = GasStructureGrid::default();
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
    let mut mass_errors = [0.0f32; 3];
    for i in 0..states.len() {
        for j in i + 1..states.len() {
            let metrics = compare_cpu_gpu_fields(&states[i], &states[j], &world, radius);
            all_errors.push(metrics.mean_abs_error);
            all_errors.push(metrics.p95_abs_error);
            all_errors.push(metrics.max_abs_error);
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

#[cfg(test)]
mod tests {
    use super::{
        parity_passes, run_cpu_cpu_parity_scenario, run_cpu_gpu_parity_scenario,
        run_cpu_only_calibration, ParityThresholds, PARITY_SCENARIOS, PARITY_STEPS,
        PARITY_THRESHOLDS,
    };
    use crate::simulation::gas::GasField;
    use crate::simulation::gpu_solver::GpuGasSolver;
    use crate::world::grid::{is_boundary, CellMaterial, WorldGrid, WORLD_HEIGHT, WORLD_WIDTH};
    use bevy::prelude::UVec2;

    fn gpu_available() -> bool {
        crate::simulation::gpu_solver::GpuGasSolver::new(
            crate::world::grid::WORLD_WIDTH,
            crate::world::grid::WORLD_HEIGHT,
        )
        .is_ok()
    }

    #[test]
    fn gpu_solver_init_probe() {
        let _ = crate::simulation::gpu_solver::GpuGasSolver::new(
            crate::world::grid::WORLD_WIDTH,
            crate::world::grid::WORLD_HEIGHT,
        );
    }

    #[test]
    fn parity_thresholds_configuration_is_sane() {
        assert!(PARITY_THRESHOLDS.radius >= 2);
        assert!(PARITY_THRESHOLDS.mean_abs_error_max > 0.0);
        assert!(PARITY_THRESHOLDS.p95_abs_error_max >= PARITY_THRESHOLDS.mean_abs_error_max);
        assert!(PARITY_THRESHOLDS.max_abs_error_max >= PARITY_THRESHOLDS.p95_abs_error_max);
    }

    #[test]
    fn parity_smoke_open_field() {
        if !gpu_available() {
            eprintln!("Skipping parity_smoke_open_field: GPU is unavailable");
            return;
        }
        let thresholds = ParityThresholds {
            radius: PARITY_THRESHOLDS.radius,
            mean_abs_error_max: PARITY_THRESHOLDS.mean_abs_error_max * 2.0,
            p95_abs_error_max: PARITY_THRESHOLDS.p95_abs_error_max * 2.0,
            max_abs_error_max: PARITY_THRESHOLDS.max_abs_error_max * 2.0,
            mass_rel_error_max: PARITY_THRESHOLDS.mass_rel_error_max * 2.0,
        };
        let metrics =
            run_cpu_gpu_parity_scenario(PARITY_SCENARIOS[0], 200, PARITY_THRESHOLDS.radius)
                .expect("smoke parity scenario");
        assert!(parity_passes(&metrics, thresholds), "metrics={metrics:?}");
    }

    #[test]
    fn wall_adjacency_gpu_does_not_create_systematic_concentration_drop() {
        if !gpu_available() {
            eprintln!(
                "Skipping wall_adjacency_gpu_does_not_create_systematic_concentration_drop: GPU is unavailable"
            );
            return;
        }

        let registry = super::test_registry_three_gases();
        let config = super::tuned_config();

        let run_locked_room = |room_size: u32| {
            let mut world = WorldGrid::default();
            let left = 40;
            let top = 40;
            let right = left + room_size - 1;
            let bottom = top + room_size - 1;

            for x in (left - 1)..=(right + 1) {
                let _ = world.set_solid_with_material(x, top - 1, CellMaterial::Brick);
                let _ = world.set_solid_with_material(x, bottom + 1, CellMaterial::Brick);
            }
            for y in (top - 1)..=(bottom + 1) {
                let _ = world.set_solid_with_material(left - 1, y, CellMaterial::Brick);
                let _ = world.set_solid_with_material(right + 1, y, CellMaterial::Brick);
            }

            let mut gpu_field = GasField::from_registry(&registry);
            gpu_field.clear_rect(
                UVec2::new(1, 1),
                UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
            );
            for y in top..=bottom {
                for x in left..=right {
                    debug_assert!(!world.is_solid(x, y) && !is_boundary(x, y));
                    gpu_field.set_amount(x, y, 0, 100.0);
                }
            }
            gpu_field.recompute_total_density_buffer(&world);

            let mut solver = GpuGasSolver::from_cpu_state(&world, &gpu_field)
                .expect("create GPU solver from CPU state")
                .0;
            let mut step = crate::simulation::SimulationStep(0);
            for _ in 0..100 {
                let params = GpuGasSolver::params_from_config(
                    &config,
                    WORLD_WIDTH,
                    WORLD_HEIGHT,
                    step.0,
                    &gpu_field,
                );
                solver.step(params).expect("gpu step");
                step.0 = step.0.saturating_add(1);
            }
            let host = solver.readback_state().expect("readback gpu state");
            gpu_field.apply_gpu_host_state(&host);

            let mut room_sum = 0.0f32;
            let mut room_n = 0u32;
            for y in top..=bottom {
                for x in left..=right {
                    room_sum += gpu_field.amount(x, y, 0);
                    room_n += 1;
                }
            }
            let room_avg = room_sum / room_n as f32;

            let corners = [(left, top), (right, top), (left, bottom), (right, bottom)];
            for (cx, cy) in corners {
                let corner_amount = gpu_field.amount(cx, cy, 0);
                assert!(
                    corner_amount >= room_avg * 0.95,
                    "GPU corner concentration is too low in locked {}x{} room at ({}, {}): corner={}, room_avg={}",
                    room_size,
                    room_size,
                    cx,
                    cy,
                    corner_amount,
                    room_avg
                );
            }
        };

        for room_size in [3u32, 5u32, 9u32] {
            run_locked_room(room_size);
        }
    }

    #[test]
    #[ignore = "Manual metrics report for CPU-vs-CPU and CPU-vs-GPU scenarios."]
    fn parity_metrics_report_cpu_cpu_and_cpu_gpu() {
        if !gpu_available() {
            eprintln!("Skipping parity_metrics_report_cpu_cpu_and_cpu_gpu: GPU is unavailable");
            return;
        }

        let steps = PARITY_STEPS;
        let radius = PARITY_THRESHOLDS.radius;
        let cpu_offsets = (0u64, 41u64);

        for scenario in PARITY_SCENARIOS {
            let cpu_cpu = run_cpu_cpu_parity_scenario(
                scenario,
                steps,
                radius,
                cpu_offsets.0,
                cpu_offsets.1,
            )
            .expect("cpu-vs-cpu scenario");
            println!(
                "CPUvsCPU [{}]: cells={}, mae={:.3}, p95={:.3}, max={:.3}, mass={:?}",
                scenario.name,
                cpu_cpu.compared_cells,
                cpu_cpu.mean_abs_error,
                cpu_cpu.p95_abs_error,
                cpu_cpu.max_abs_error,
                cpu_cpu.mass_rel_errors
            );

            let cpu_gpu =
                run_cpu_gpu_parity_scenario(scenario, steps, radius).expect("cpu-vs-gpu scenario");
            println!(
                "CPUvsGPU [{}]: cells={}, mae={:.3}, p95={:.3}, max={:.3}, mass={:?}",
                scenario.name,
                cpu_gpu.compared_cells,
                cpu_gpu.mean_abs_error,
                cpu_gpu.p95_abs_error,
                cpu_gpu.max_abs_error,
                cpu_gpu.mass_rel_errors
            );
        }
    }

    #[test]
    #[ignore = "Long-running parity check. Run explicitly before full performance benchmark."]
    fn parity_two_scenarios_5000_steps() {
        if !gpu_available() {
            eprintln!("Skipping parity_two_scenarios_5000_steps: GPU is unavailable");
            return;
        }
        for scenario in PARITY_SCENARIOS {
            let metrics =
                run_cpu_gpu_parity_scenario(scenario, PARITY_STEPS, PARITY_THRESHOLDS.radius)
                    .expect("parity scenario run");
            assert!(
                parity_passes(&metrics, PARITY_THRESHOLDS),
                "Scenario '{}' failed parity: {:?}",
                scenario.name,
                metrics
            );
        }
    }

    #[test]
    #[ignore = "Manual one-time calibration. Run explicitly when simulation math changes."]
    fn calibrate_cpu_only_radius_and_thresholds() {
        let offsets = [0u64, 7, 41, 117];
        let r2 = run_cpu_only_calibration(&offsets, 2).expect("cpu calibration r2");
        let r3 = run_cpu_only_calibration(&offsets, 3).expect("cpu calibration r3");
        println!(
            "CPU-only calibration | R=2 => mae={:.4}, p95={:.4}, max={:.4}, mass={:?}; \
R=3 => mae={:.4}, p95={:.4}, max={:.4}, mass={:?}",
            r2.mean_abs_error,
            r2.p95_abs_error,
            r2.max_abs_error,
            r2.mass_rel_errors,
            r3.mean_abs_error,
            r3.p95_abs_error,
            r3.max_abs_error,
            r3.mass_rel_errors
        );
    }
}
