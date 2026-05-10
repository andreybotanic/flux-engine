#[cfg(test)]
mod tests {
    use super::{
        compare_cpu_gpu_fields, parity_passes, run_cpu_cpu_parity_scenario, run_cpu_gpu_parity_scenario,
        run_cpu_only_calibration, ParityThresholds, PARITY_SCENARIOS, PARITY_STEPS,
        PARITY_THRESHOLDS,
    };
    use crate::plugins::default_plugin::pipe_runtime::{
        apply_pipe_network_step, PipeFlowVisualState, PipeFluxField, PipeGasField,
        PipeSimulationConfig,
    };
    use crate::simulation::{
        gas::GasField,
    };
    use crate::simulation::gpu_solver::GpuGasSolver;
    use crate::world::{
        grid::{is_boundary, CellMaterial, WorldGrid, WORLD_HEIGHT, WORLD_WIDTH},
        structures::PlacedStructureMap,
    };
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
    fn pipe_runtime_cpu_gpu_smoke_stays_aligned() {
        if !gpu_available() {
            eprintln!("Skipping pipe_runtime_cpu_gpu_smoke_stays_aligned: GPU is unavailable");
            return;
        }

        let registry = super::test_registry_three_gases();
        let config = super::tuned_config();
        let pipe_config = PipeSimulationConfig::default();
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        for cell in [
            UVec2::new(30, 30),
            UVec2::new(31, 30),
            UVec2::new(32, 30),
            UVec2::new(31, 29),
        ] {
            assert!(structures.place_pipe(cell.x, cell.y, &world));
        }
        assert!(structures.place_vent(30, 30, &world));
        assert!(structures.place_vent(32, 30, &world));
        assert!(structures.place_vent(31, 29, &world));

        let mut cpu_field = GasField::from_registry(&registry);
        cpu_field.set_amount(30, 30, 0, 180.0);
        cpu_field.set_amount(31, 29, 1, 40.0);
        cpu_field.recompute_total_density_buffer(&world);
        let mut gpu_field = cpu_field.clone();
        let mut cpu_pipe = PipeGasField::from_registry(&registry);
        let mut gpu_pipe = PipeGasField::from_registry(&registry);
        let mut cpu_flux = PipeFluxField::default();
        let mut gpu_flux = PipeFluxField::default();
        cpu_pipe.sync_to_structures(&structures);
        gpu_pipe.sync_to_structures(&structures);
        let mut cpu_visuals = PipeFlowVisualState::default();
        let mut gpu_visuals = PipeFlowVisualState::default();
        let mut cpu_step = crate::simulation::SimulationStep(0);
        let mut gpu_step = crate::simulation::SimulationStep(0);
        let mut block_sync = crate::simulation::BlockSyncState;
        let mut solver = GpuGasSolver::from_cpu_state(&world, &gpu_field)
            .expect("gpu solver from initial state")
            .0;

        for _ in 0..8 {
            let _ = apply_pipe_network_step(
                &structures,
                &mut cpu_pipe,
                &mut cpu_flux,
                &mut cpu_field,
                &world,
                &mut cpu_visuals,
                &pipe_config,
            );
            crate::simulation::do_one_substep(
                &mut block_sync,
                &mut cpu_field,
                &world,
                &config,
                &mut cpu_step,
            );

            let changed = apply_pipe_network_step(
                &structures,
                &mut gpu_pipe,
                &mut gpu_flux,
                &mut gpu_field,
                &world,
                &mut gpu_visuals,
                &pipe_config,
            );
            if changed {
                let _ = solver
                    .upload_state(&gpu_field.to_gpu_host_state(&world))
                    .expect("upload gpu state after pipe pre-step");
            }
            let params = GpuGasSolver::params_from_config(
                &config,
                WORLD_WIDTH,
                WORLD_HEIGHT,
                gpu_step.0,
                &gpu_field,
            );
            solver.step(params).expect("gpu step");
            gpu_step.0 = gpu_step.0.saturating_add(1);
        }

        let host = solver.readback_state().expect("readback gpu state");
        gpu_field.apply_gpu_host_state(&host);
        let metrics = compare_cpu_gpu_fields(&cpu_field, &gpu_field, &world, PARITY_THRESHOLDS.radius);
        assert!(
            parity_passes(
                &metrics,
                ParityThresholds {
                    radius: PARITY_THRESHOLDS.radius,
                    mean_abs_error_max: PARITY_THRESHOLDS.mean_abs_error_max * 2.5,
                    p95_abs_error_max: PARITY_THRESHOLDS.p95_abs_error_max * 2.5,
                    max_abs_error_max: PARITY_THRESHOLDS.max_abs_error_max * 2.5,
                    mass_rel_error_max: PARITY_THRESHOLDS.mass_rel_error_max * 2.5,
                }
            ),
            "pipe parity metrics={metrics:?}"
        );
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
