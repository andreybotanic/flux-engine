#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{GameConfig, GasDefinition};
    use crate::save::{list_saves, load_save, saves_root_default};
    use crate::simulation::{do_one_substep, BlockSyncState, GasSimulationConfig, SimulationStep};

    fn registry_with_three() -> GasRegistry {
        GasRegistry::new(vec![
            GasDefinition {
                id: "h2".to_string(),
                label: "Hydrogen".to_string(),
                molecular_mass: 2.016,
                color: [0.8, 0.2, 0.9],
            },
            GasDefinition {
                id: "o2".to_string(),
                label: "Oxygen".to_string(),
                molecular_mass: 31.998,
                color: [0.0, 0.8, 0.8],
            },
            GasDefinition {
                id: "co2".to_string(),
                label: "Carbon dioxide".to_string(),
                molecular_mass: 44.009,
                color: [0.5, 0.5, 0.5],
            },
        ])
        .expect("valid registry")
    }

    fn registry_with_four() -> GasRegistry {
        GasRegistry::new(vec![
            GasDefinition {
                id: "h2".to_string(),
                label: "Hydrogen".to_string(),
                molecular_mass: 2.016,
                color: [0.8, 0.2, 0.9],
            },
            GasDefinition {
                id: "o2".to_string(),
                label: "Oxygen".to_string(),
                molecular_mass: 31.998,
                color: [0.0, 0.8, 0.8],
            },
            GasDefinition {
                id: "co2".to_string(),
                label: "Carbon dioxide".to_string(),
                molecular_mass: 44.009,
                color: [0.5, 0.5, 0.5],
            },
            GasDefinition {
                id: "n2".to_string(),
                label: "Nitrogen".to_string(),
                molecular_mass: 28.014,
                color: [0.3, 0.7, 1.0],
            },
        ])
        .expect("valid registry")
    }

    fn inhomogeneity_metric(field: &GasField, world: &WorldGrid) -> f32 {
        let mut acc = 0.0f32;
        let mut count = 0u32;
        for y in 2..WORLD_HEIGHT - 2 {
            for x in 2..WORLD_WIDTH - 2 {
                if world.is_solid(x, y) || is_boundary(x, y) {
                    continue;
                }
                let c = field.total_amount(x, y);
                let n_mean = 0.25
                    * (field.total_amount(x - 1, y)
                        + field.total_amount(x + 1, y)
                        + field.total_amount(x, y - 1)
                        + field.total_amount(x, y + 1));
                acc += (c - n_mean).abs();
                count += 1;
            }
        }
        if count == 0 {
            0.0
        } else {
            acc / count as f32
        }
    }

    fn species_center_y(field: &GasField, world: &WorldGrid, gas_index: usize) -> f32 {
        let mut sum_mass = 0.0f64;
        let mut sum_y_mass = 0.0f64;
        for y in 1..WORLD_HEIGHT - 1 {
            for x in 1..WORLD_WIDTH - 1 {
                if world.is_solid(x, y) || is_boundary(x, y) {
                    continue;
                }
                let mass = field.amount_particles(x, y, gas_index) as f64;
                sum_mass += mass;
                sum_y_mass += mass * (y as f64);
            }
        }
        if sum_mass <= 0.0 {
            0.0
        } else {
            (sum_y_mass / sum_mass) as f32
        }
    }

    struct RowBandStats {
        avg: f32,
        min: u32,
        max: u32,
        n: usize,
    }

    fn band_stats(values: &[u32]) -> Result<RowBandStats, String> {
        if values.is_empty() {
            return Err("empty sampling band".to_string());
        }
        let sum: u64 = values.iter().map(|&v| u64::from(v)).sum();
        let min = *values.iter().min().unwrap_or(&0);
        let max = *values.iter().max().unwrap_or(&0);
        Ok(RowBandStats {
            avg: sum as f32 / values.len() as f32,
            min,
            max,
            n: values.len(),
        })
    }

    fn hydrogen_inside_outside_equals_row43_after_steps(
        steps: u64,
        row_y: u32,
    ) -> Result<(RowBandStats, RowBandStats), String> {
        let game_cfg = GameConfig::load_from_default_location()?;
        let registry = game_cfg.gas_registry.clone();
        let h2_index = registry
            .index_of("h2")
            .ok_or_else(|| "Registry does not contain gas id 'h2'".to_string())?;

        let saves_root = saves_root_default();
        let saves = list_saves(&saves_root).map_err(|e| e.to_string())?;
        let equals = saves
            .iter()
            .find(|s| s.display_name == "equals")
            .ok_or_else(|| {
                format!(
                    "Save with display_name='equals' not found in '{}'",
                    saves_root.display()
                )
            })?;
        let loaded = load_save(&saves_root, &equals.id, &registry).map_err(|e| e.to_string())?;

        let mut world = WorldGrid::default();
        world
            .restore_from_cell_codes(&loaded.state.world_cell_codes)
            .map_err(|e| format!("World restore failed: {}", e))?;
        let mut field = GasField::from_registry(&registry);
        field
            .restore_state(&loaded.state.gas_snapshot)
            .map_err(|e| format!("Gas restore failed: {}", e))?;

        let mut block = BlockSyncState;
        let mut step = SimulationStep(loaded.state.simulation_step);
        let cfg = game_cfg.gas_simulation;
        for _ in 0..steps {
            do_one_substep(&mut block, &mut field, &world, &cfg, &mut step);
        }

        let y = row_y;
        let inside_min_x = 36u32;
        let inside_max_x = 64u32;

        let mut inside_values = Vec::new();
        let mut outside_values_user = Vec::new();

        for x in 1..WORLD_WIDTH - 1 {
            if world.is_solid(x, y) || is_boundary(x, y) {
                continue;
            }
            let v = field.amount_particles(x, y, h2_index);
            if x >= inside_min_x && x <= inside_max_x {
                inside_values.push(v);
            } else {
                // User-defined "outside": row 43, columns 1..34 and 66..100.
                if (1..=34).contains(&x) || (66..=100).contains(&x) {
                    outside_values_user.push(v);
                }
            }
        }

        let inside = band_stats(&inside_values)?;
        let outside_user = band_stats(&outside_values_user)?;
        Ok((inside, outside_user))
    }

    #[test]
    fn creates_from_registry_with_empty_initial_cells() {
        let registry = registry_with_three();
        let field = GasField::from_registry(&registry);
        let cx = WORLD_WIDTH / 2;
        let cy = WORLD_HEIGHT / 2;
        assert_eq!(field.gas_count(), 3);
        assert_eq!(field.amount_rounded(cx, cy, 0), 0);
        assert_eq!(field.amount_rounded(cx, cy, 1), 0);
        assert_eq!(field.amount_rounded(cx, cy, 2), 0);
    }

    #[test]
    fn totals_include_all_active_species() {
        let registry = registry_with_three();
        let mut field = GasField::from_registry(&registry);
        field.clear_cell(10, 10);
        field.set_amount(10, 10, 0, 5.0);
        field.set_amount(10, 10, 1, 7.0);
        field.set_amount(10, 10, 2, 11.0);
        assert_eq!(field.total_amount(10, 10), 23.0);
    }

    #[test]
    fn gpu_host_state_preserves_dynamic_gas_count_without_truncation() {
        let registry = registry_with_four();
        let world = WorldGrid::default();
        let mut field = GasField::from_registry(&registry);
        field.clear_cell(10, 10);
        field.set_amount(10, 10, 0, 1.0);
        field.set_amount(10, 10, 1, 2.0);
        field.set_amount(10, 10, 2, 3.0);
        field.set_amount(10, 10, 3, 4.0);
        field.recompute_total_density_buffer(&world);

        let host = field.to_gpu_host_state(&world);
        let cells = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
        assert_eq!(host.gas_count, 4);
        assert_eq!(host.molecular_masses.len(), 4);
        assert_eq!(host.species.len(), cells * 4);

        let idx = linear_index(10, 10);
        let base = idx * 4;
        assert_eq!(host.species[base], 1);
        assert_eq!(host.species[base + 1], 2);
        assert_eq!(host.species[base + 2], 3);
        assert_eq!(host.species[base + 3], 4);
    }

    #[test]
    fn particles_stay_integer_and_mass_is_exact() {
        let registry = registry_with_three();
        let world = WorldGrid::default();
        let mut field = GasField::from_registry(&registry);
        field.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );
        field.set_amount(WORLD_WIDTH / 2, WORLD_HEIGHT / 2, 0, 1234.0);
        field.set_amount(WORLD_WIDTH / 2 + 1, WORLD_HEIGHT / 2, 1, 999.0);
        field.set_amount(WORLD_WIDTH / 2, WORLD_HEIGHT / 2 + 1, 2, 321.0);

        let cfg = GasSimulationConfig::default();
        let mut step = SimulationStep(0);
        let mut block = BlockSyncState;
        let base = field.species_totals_u64(&world);

        for _ in 0..200 {
            do_one_substep(&mut block, &mut field, &world, &cfg, &mut step);
            for y in 1..WORLD_HEIGHT - 1 {
                for x in 1..WORLD_WIDTH - 1 {
                    for g in 0..field.gas_count() {
                        let _v: u32 = field.amount_particles(x, y, g);
                    }
                }
            }
            assert_eq!(base, field.species_totals_u64(&world));
        }
    }

    #[test]
    fn single_particle_moves_only_to_neighbors() {
        let registry = registry_with_three();
        let world = WorldGrid::default();
        let mut field = GasField::from_registry(&registry);
        field.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );

        let cx = WORLD_WIDTH / 2;
        let cy = WORLD_HEIGHT / 2;
        field.set_amount(cx, cy, 0, 1.0);

        let cfg = GasSimulationConfig::default();
        let mut step = SimulationStep(0);
        let mut block = BlockSyncState;
        do_one_substep(&mut block, &mut field, &world, &cfg, &mut step);

        for y in 1..WORLD_HEIGHT - 1 {
            for x in 1..WORLD_WIDTH - 1 {
                let amount = field.amount_particles(x, y, 0);
                if amount == 0 {
                    continue;
                }
                let dx = x.abs_diff(cx);
                let dy = y.abs_diff(cy);
                assert!(
                    dx + dy <= 1,
                    "particle teleported to ({}, {}), start=({}, {})",
                    x,
                    y,
                    cx,
                    cy
                );
            }
        }
    }

    #[test]
    fn buoyancy_separates_mixture_by_mass() {
        let registry = registry_with_three();
        let world = WorldGrid::default();
        let mut field = GasField::from_registry(&registry);
        field.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );

        for y in 36..66 {
            for x in 36..66 {
                field.set_amount(x, y, 0, 30.0);
                field.set_amount(x, y, 1, 30.0);
                field.set_amount(x, y, 2, 30.0);
            }
        }

        let cfg = GasSimulationConfig::default();
        let mut step = SimulationStep(0);
        let mut block = BlockSyncState;
        let base = field.species_totals_u64(&world);
        for _ in 0..700 {
            do_one_substep(&mut block, &mut field, &world, &cfg, &mut step);
        }

        let y_h2 = species_center_y(&field, &world, 0);
        let y_o2 = species_center_y(&field, &world, 1);
        let y_co2 = species_center_y(&field, &world, 2);

        assert!(
            y_h2 > y_o2 + 0.5,
            "expected H2 above O2, got y_h2={} y_o2={}",
            y_h2,
            y_o2
        );
        assert!(
            y_o2 > y_co2 + 0.5,
            "expected O2 above CO2, got y_o2={} y_co2={}",
            y_o2,
            y_co2
        );
        assert_eq!(base, field.species_totals_u64(&world));
    }

    #[test]
    fn configurations_relax_to_near_equilibrium_with_small_fluctuations() {
        let registry = registry_with_three();
        let world = WorldGrid::default();
        let scenarios = [0u32, 1u32, 2u32, 3u32];

        for scenario in scenarios {
            let mut field = GasField::from_registry(&registry);
            field.clear_rect(
                UVec2::new(1, 1),
                UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
            );

            match scenario {
                0 => {
                    field.set_amount(WORLD_WIDTH / 2, WORLD_HEIGHT / 2, 0, 40_000.0);
                    field.set_amount(WORLD_WIDTH / 2 + 1, WORLD_HEIGHT / 2, 1, 15_000.0);
                }
                1 => {
                    for y in 1..WORLD_HEIGHT - 1 {
                        for x in 1..WORLD_WIDTH - 1 {
                            if (x + y) % 3 == 0 {
                                field.set_amount(x, y, 0, 20.0);
                            }
                            if (x + 2 * y) % 5 == 0 {
                                field.set_amount(x, y, 1, 15.0);
                            }
                        }
                    }
                }
                2 => {
                    for y in 1..WORLD_HEIGHT - 1 {
                        let amount = if y % 2 == 0 { 25.0 } else { 5.0 };
                        for x in 1..WORLD_WIDTH - 1 {
                            field.set_amount(x, y, 2, amount);
                        }
                    }
                }
                _ => {
                    for y in 20..82 {
                        for x in 20..82 {
                            let a0 = ((x * 17 + y * 31) % 41) as f32;
                            let a1 = ((x * 11 + y * 13) % 29) as f32;
                            field.set_amount(x, y, 0, a0);
                            field.set_amount(x, y, 1, a1);
                        }
                    }
                }
            }

            let cfg = GasSimulationConfig::default();
            let mut step = SimulationStep(0);
            let mut block = BlockSyncState;
            let base = field.species_totals_u64(&world);

            let metric_start = inhomogeneity_metric(&field, &world);
            let mut late_window = Vec::new();
            for i in 0..900 {
                do_one_substep(&mut block, &mut field, &world, &cfg, &mut step);
                if i >= 600 {
                    late_window.push(inhomogeneity_metric(&field, &world));
                }
            }
            let metric_end = inhomogeneity_metric(&field, &world);
            assert!(
                metric_end < metric_start * 0.75,
                "scenario {} did not relax enough: start={}, end={}",
                scenario,
                metric_start,
                metric_end
            );

            let mid = late_window.len() / 2;
            let mean_a = late_window[..mid].iter().sum::<f32>() / mid as f32;
            let mean_b = late_window[mid..].iter().sum::<f32>() / (late_window.len() - mid) as f32;
            assert!(
                (mean_b - mean_a).abs() <= metric_start * 0.06,
                "scenario {} late trend too large: mean_a={}, mean_b={}, start={}",
                scenario,
                mean_a,
                mean_b,
                metric_start
            );

            let late_mean = late_window.iter().sum::<f32>() / late_window.len() as f32;
            let late_var = late_window
                .iter()
                .map(|v| {
                    let d = *v - late_mean;
                    d * d
                })
                .sum::<f32>()
                / late_window.len() as f32;
            let late_std = late_var.sqrt();
            assert!(
                late_std > 0.0 && late_std <= (late_mean * 0.35 + 1e-4),
                "scenario {} fluctuation envelope invalid: mean={}, std={}",
                scenario,
                late_mean,
                late_std
            );
            assert_eq!(base, field.species_totals_u64(&world));
        }
    }

    #[test]
    fn wall_adjacency_does_not_create_systematic_concentration_drop() {
        let registry = registry_with_three();
        let mut world = WorldGrid::default();
        // Inner rectangular wall.
        for x in 20..=80 {
            let _ = world.set_solid_with_material(x, 20, crate::world::grid::CellMaterial::Brick);
            let _ = world.set_solid_with_material(x, 80, crate::world::grid::CellMaterial::Brick);
        }
        for y in 20..=80 {
            let _ = world.set_solid_with_material(20, y, crate::world::grid::CellMaterial::Brick);
            let _ = world.set_solid_with_material(80, y, crate::world::grid::CellMaterial::Brick);
        }

        let mut field = GasField::from_registry(&registry);
        field.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );

        // Uniform fill in open cells for one species.
        for y in 1..WORLD_HEIGHT - 1 {
            for x in 1..WORLD_WIDTH - 1 {
                if world.is_solid(x, y) || is_boundary(x, y) {
                    continue;
                }
                field.set_amount(x, y, 0, 200.0);
            }
        }

        let cfg = GasSimulationConfig::default();
        let mut step = SimulationStep(0);
        let mut block = BlockSyncState;
        for _ in 0..400 {
            do_one_substep(&mut block, &mut field, &world, &cfg, &mut step);
        }

        let mut near_sum = 0.0f32;
        let mut near_n = 0u32;
        let mut far_sum = 0.0f32;
        let mut far_n = 0u32;

        for y in 1..WORLD_HEIGHT - 1 {
            for x in 1..WORLD_WIDTH - 1 {
                if world.is_solid(x, y) || is_boundary(x, y) {
                    continue;
                }
                let near_inner_wall = (x >= 21 && x <= 79 && (y == 21 || y == 79))
                    || (y >= 21 && y <= 79 && (x == 21 || x == 79));
                let far_from_inner_wall = x >= 30 && x <= 70 && y >= 30 && y <= 70;
                if near_inner_wall {
                    near_sum += field.amount(x, y, 0);
                    near_n += 1;
                } else if far_from_inner_wall {
                    far_sum += field.amount(x, y, 0);
                    far_n += 1;
                }
            }
        }

        let near_avg = near_sum / near_n as f32;
        let far_avg = far_sum / far_n as f32;
        // Allow small Brownian fluctuations, but forbid persistent wall depletion.
        assert!(
            near_avg >= far_avg * 0.95,
            "wall-adjacent concentration is too low: near_avg={}, far_avg={}",
            near_avg,
            far_avg
        );
    }

    #[test]
    fn buoyancy_context_does_not_see_through_vertical_wall() {
        let registry = registry_with_three();
        let mut world_blocked = WorldGrid::default();
        let world_open = WorldGrid::default();

        for y in 10..=90 {
            let _ = world_blocked.set_solid_with_material(
                50,
                y,
                crate::world::grid::CellMaterial::Brick,
            );
        }

        let mut field_blocked = GasField::from_registry(&registry);
        let mut field_open = GasField::from_registry(&registry);
        field_blocked.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );
        field_open.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );

        for y in 1..WORLD_HEIGHT - 1 {
            for x in 1..WORLD_WIDTH - 1 {
                if !is_boundary(x, y) && !world_blocked.is_solid(x, y) {
                    if x <= 49 {
                        field_blocked.set_amount(x, y, 0, 100.0);
                    } else {
                        field_blocked.set_amount(x, y, 2, 100.0);
                    }
                }

                if !is_boundary(x, y) && !world_open.is_solid(x, y) {
                    if x <= 49 {
                        field_open.set_amount(x, y, 0, 100.0);
                    } else {
                        field_open.set_amount(x, y, 2, 100.0);
                    }
                }
            }
        }

        let kernel = kernel_offsets_for_radius(2);
        let sigma = 1.2f32;
        let inv_two_sigma_sq = 1.0 / (2.0 * sigma * sigma);
        let kernel_weights: Vec<f32> = kernel
            .iter()
            .map(|sample| (-sample.dist2 * inv_two_sigma_sq).exp())
            .collect();

        let blocked_env = estimate_local_env_mix_mass(
            &field_blocked,
            &world_blocked,
            49,
            50,
            kernel,
            &kernel_weights,
        )
        .expect("blocked env mass");
        let open_env =
            estimate_local_env_mix_mass(&field_open, &world_open, 49, 50, kernel, &kernel_weights)
                .expect("open env mass");

        assert!(
            blocked_env < 10.0,
            "blocked side should stay close to light-gas mass, got {}",
            blocked_env
        );
        assert!(
            open_env > 15.0,
            "without wall occlusion local env must include heavy side, got {}",
            open_env
        );
    }

    #[test]
    fn buoyancy_context_does_not_see_diagonal_through_corner_walls() {
        let registry = registry_with_three();
        let mut world_blocked = WorldGrid::default();
        let world_open = WorldGrid::default();

        let center_x = 50;
        let center_y = 50;
        let _ = world_blocked.set_solid_with_material(
            center_x,
            center_y - 1,
            crate::world::grid::CellMaterial::Brick,
        );
        let _ = world_blocked.set_solid_with_material(
            center_x,
            center_y + 1,
            crate::world::grid::CellMaterial::Brick,
        );
        let _ = world_blocked.set_solid_with_material(
            center_x - 1,
            center_y,
            crate::world::grid::CellMaterial::Brick,
        );
        let _ = world_blocked.set_solid_with_material(
            center_x + 1,
            center_y,
            crate::world::grid::CellMaterial::Brick,
        );

        let mut field_blocked = GasField::from_registry(&registry);
        let mut field_open = GasField::from_registry(&registry);
        field_blocked.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );
        field_open.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );

        field_blocked.set_amount(center_x, center_y, 0, 100.0);
        field_open.set_amount(center_x, center_y, 0, 100.0);

        let diagonals = [
            (center_x - 1, center_y - 1),
            (center_x - 1, center_y + 1),
            (center_x + 1, center_y - 1),
            (center_x + 1, center_y + 1),
        ];
        for (dx, dy) in diagonals {
            field_blocked.set_amount(dx, dy, 2, 100.0);
            field_open.set_amount(dx, dy, 2, 100.0);
        }

        let kernel = kernel_offsets_for_radius(2);
        let sigma = 1.2f32;
        let inv_two_sigma_sq = 1.0 / (2.0 * sigma * sigma);
        let kernel_weights: Vec<f32> = kernel
            .iter()
            .map(|sample| (-sample.dist2 * inv_two_sigma_sq).exp())
            .collect();

        let blocked_env = estimate_local_env_mix_mass(
            &field_blocked,
            &world_blocked,
            center_x,
            center_y,
            kernel,
            &kernel_weights,
        );
        let open_env = estimate_local_env_mix_mass(
            &field_open,
            &world_open,
            center_x,
            center_y,
            kernel,
            &kernel_weights,
        )
        .expect("open env mass");

        assert!(
            blocked_env.is_none(),
            "fully enclosed center should have no reachable buoyancy samples, got {:?}",
            blocked_env
        );
        assert!(
            open_env > 15.0,
            "without corner walls diagonal heavy gas should affect env mass, got {}",
            open_env
        );
    }

    #[test]
    fn gas_snapshot_roundtrip_preserves_internal_state() {
        let registry = registry_with_three();
        let world = WorldGrid::default();
        let mut field = GasField::from_registry(&registry);
        field.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );
        let cx = WORLD_WIDTH / 2;
        let cy = WORLD_HEIGHT / 2;
        let _ = field.apply_species_delta_with_lbm(cx, cy, 0, 6000.0);
        let _ = field.apply_species_delta_with_lbm(cx + 1, cy, 1, 3000.0);
        let _ = field.apply_species_delta_with_lbm(cx, cy + 1, 2, 1000.0);
        field.recompute_total_density_buffer(&world);

        let snapshot = field.snapshot_state();
        let mut restored = GasField::from_registry(&registry);
        restored
            .restore_state(&snapshot)
            .expect("restore from valid snapshot");

        let original_snapshot = field.snapshot_state();
        let restored_snapshot = restored.snapshot_state();
        assert_eq!(original_snapshot.gas_count, restored_snapshot.gas_count);
        assert_eq!(original_snapshot.species, restored_snapshot.species);
        assert_eq!(
            original_snapshot.total_density.len(),
            restored_snapshot.total_density.len()
        );
        assert_eq!(
            original_snapshot.velocity.len(),
            restored_snapshot.velocity.len()
        );

        for i in 0..original_snapshot.total_density.len() {
            assert!(
                (original_snapshot.total_density[i] - restored_snapshot.total_density[i]).abs()
                    < 1e-6,
                "total_density mismatch at index {}",
                i
            );
            assert!(
                (original_snapshot.velocity[i][0] - restored_snapshot.velocity[i][0]).abs() < 1e-6,
                "velocity.x mismatch at index {}",
                i
            );
            assert!(
                (original_snapshot.velocity[i][1] - restored_snapshot.velocity[i][1]).abs() < 1e-6,
                "velocity.y mismatch at index {}",
                i
            );
        }
    }

    #[test]
    #[ignore = "long-running scenario check for tuning against the 'equals' save"]
    fn equals_inverted_cup_hydrogen_retention_after_50k_steps() {
        let (inside, outside_user_50k) =
            hydrogen_inside_outside_equals_row43_after_steps(50_000, 43)
                .expect("scenario must run");
        let (inside_70k, outside_user_70k) =
            hydrogen_inside_outside_equals_row43_after_steps(70_000, 43)
                .expect("scenario must run");
        let mirrored_row = (WORLD_HEIGHT - 1).saturating_sub(43);
        let (inside_70k_m, outside_70k_m) =
            hydrogen_inside_outside_equals_row43_after_steps(70_000, mirrored_row)
                .expect("scenario must run");
        let ratio_user_50k = inside.avg / outside_user_50k.avg.max(1e-6);
        let ratio_user_70k = inside_70k.avg / outside_user_70k.avg.max(1e-6);
        let ratio_user_70k_m = inside_70k_m.avg / outside_70k_m.avg.max(1e-6);
        println!(
            "equals row43 H2 @50k: inside avg={} min={} max={} (n={}), outside_user avg={} min={} max={} (n={}), ratio_user_50k={}; @70k: inside avg={} min={} max={} (n={}), outside_user avg={} min={} max={} (n={}), ratio_user_70k={}; mirrored_row(y={}) @70k: inside avg={} min={} max={} (n={}), outside_user avg={} min={} max={} (n={}), ratio_user_70k_m={}",
            inside.avg,
            inside.min,
            inside.max,
            inside.n,
            outside_user_50k.avg,
            outside_user_50k.min,
            outside_user_50k.max,
            outside_user_50k.n,
            ratio_user_50k,
            inside_70k.avg,
            inside_70k.min,
            inside_70k.max,
            inside_70k.n,
            outside_user_70k.avg,
            outside_user_70k.min,
            outside_user_70k.max,
            outside_user_70k.n,
            ratio_user_70k,
            mirrored_row,
            inside_70k_m.avg,
            inside_70k_m.min,
            inside_70k_m.max,
            inside_70k_m.n,
            outside_70k_m.avg,
            outside_70k_m.min,
            outside_70k_m.max,
            outside_70k_m.n,
            ratio_user_70k_m
        );
        assert!(
            ratio_user_50k > 1.0,
            "expected user-defined inverted-cup H2 retention at row 43 after 50k: ratio_user_50k={} (inside_avg={}, outside_user_avg={})",
            ratio_user_50k,
            inside.avg,
            outside_user_50k.avg
        );
    }
}
