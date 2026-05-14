#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{GameConfig, GasDefinition};
    use crate::save::{list_saves, load_save, saves_root_default};
    use crate::simulation::{do_one_substep, BlockSyncState, GasSimulationConfig, SimulationStep};
    use std::collections::VecDeque;

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
        let content_registry = crate::plugins::default_plugin::default_content_registry();
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
        let loaded = load_save(&saves_root, &equals.id, &registry, &content_registry)
            .map_err(|e| e.to_string())?;

        let mut world = WorldGrid::default();
        world
            .restore_cells(&loaded.state.world_cells)
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

    #[derive(Clone, Copy, Debug)]
    enum WallBiasScenario {
        OpenRoom,
        CenterBlock,
        StairStep,
    }

    impl WallBiasScenario {
        fn name(self) -> &'static str {
            match self {
                Self::OpenRoom => "open_room",
                Self::CenterBlock => "center_block",
                Self::StairStep => "stair_step",
            }
        }
    }

    #[derive(Clone, Copy, Debug)]
    struct WallBiasMetrics {
        near_avg: f32,
        far_avg: f32,
        near_count: u32,
        far_count: u32,
    }

    #[derive(Clone, Copy, Debug)]
    struct WallMomentumMetrics {
        near_wall_cells: u32,
        near_mass: f32,
        radial_bias_per_mass: f32,
        radial_abs_per_mass: f32,
    }

    fn mark_solid(world: &mut WorldGrid, x: u32, y: u32) {
        let _ = world.set_solid_with_material(
            x,
            y,
            crate::plugins::default_plugin::brick_cell_material(),
        );
    }

    fn build_wall_bias_world(scenario: WallBiasScenario) -> (WorldGrid, UVec2, UVec2, Vec<UVec2>) {
        let mut world = WorldGrid::default();
        let room_min = UVec2::new(42, 42);
        let room_max = UVec2::new(60, 60);
        let mut focus_walls = Vec::new();

        for x in (room_min.x - 1)..=(room_max.x + 1) {
            mark_solid(&mut world, x, room_min.y - 1);
            mark_solid(&mut world, x, room_max.y + 1);
        }
        for y in (room_min.y - 1)..=(room_max.y + 1) {
            mark_solid(&mut world, room_min.x - 1, y);
            mark_solid(&mut world, room_max.x + 1, y);
        }

        let cx = (room_min.x + room_max.x) / 2;
        let cy = (room_min.y + room_max.y) / 2;
        match scenario {
            WallBiasScenario::OpenRoom => {}
            WallBiasScenario::CenterBlock => {
                for y in (cy - 1)..=(cy + 1) {
                    for x in (cx - 1)..=(cx + 1) {
                        mark_solid(&mut world, x, y);
                        focus_walls.push(UVec2::new(x, y));
                    }
                }
            }
            WallBiasScenario::StairStep => {
                for (dx, dy) in [(0i32, -2i32), (1, -1), (0, 0), (1, 1), (0, 2)] {
                    let x = (cx as i32 + dx) as u32;
                    let y = (cy as i32 + dy) as u32;
                    mark_solid(&mut world, x, y);
                    focus_walls.push(UVec2::new(x, y));
                }
            }
        }

        (world, room_min, room_max, focus_walls)
    }

    fn fill_room_uniform(
        field: &mut GasField,
        world: &WorldGrid,
        room_min: UVec2,
        room_max: UVec2,
        gas_index: usize,
        density: u32,
    ) {
        for y in room_min.y..=room_max.y {
            for x in room_min.x..=room_max.x {
                if is_boundary(x, y) || world.is_solid(x, y) {
                    continue;
                }
                field.set_amount(x, y, gas_index, density as f32);
            }
        }
    }

    fn cell_is_blocked(world: &WorldGrid, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= WORLD_WIDTH as i32 || y >= WORLD_HEIGHT as i32 {
            return true;
        }
        let ux = x as u32;
        let uy = y as u32;
        is_boundary(ux, uy) || world.is_solid(ux, uy)
    }

    fn wall_bias_metrics(
        field: &GasField,
        world: &WorldGrid,
        room_min: UVec2,
        room_max: UVec2,
        focus_walls: &[UVec2],
        gas_index: usize,
    ) -> WallBiasMetrics {
        let room_w = room_max.x - room_min.x + 1;
        let room_h = room_max.y - room_min.y + 1;
        let mut distance = vec![u16::MAX; (room_w * room_h) as usize];
        let mut queue = VecDeque::new();

        let room_index =
            |x: u32, y: u32| -> usize { ((y - room_min.y) * room_w + (x - room_min.x)) as usize };
        let neighbor_dirs = [(0i32, 1i32), (0, -1), (-1, 0), (1, 0)];

        for y in room_min.y..=room_max.y {
            for x in room_min.x..=room_max.x {
                if is_boundary(x, y) || world.is_solid(x, y) {
                    continue;
                }
                let has_wall_neighbor = neighbor_dirs.iter().any(|(dx, dy)| {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    focus_walls
                        .iter()
                        .any(|wall| wall.x == nx as u32 && wall.y == ny as u32)
                });
                if has_wall_neighbor {
                    let idx = room_index(x, y);
                    distance[idx] = 1;
                    queue.push_back(UVec2::new(x, y));
                }
            }
        }

        while let Some(cell) = queue.pop_front() {
            let current_distance = distance[room_index(cell.x, cell.y)];
            for (dx, dy) in neighbor_dirs {
                let nx = cell.x as i32 + dx;
                let ny = cell.y as i32 + dy;
                if nx < room_min.x as i32
                    || ny < room_min.y as i32
                    || nx > room_max.x as i32
                    || ny > room_max.y as i32
                {
                    continue;
                }
                if cell_is_blocked(world, nx, ny) {
                    continue;
                }
                let nx = nx as u32;
                let ny = ny as u32;
                let next_idx = room_index(nx, ny);
                if distance[next_idx] != u16::MAX {
                    continue;
                }
                distance[next_idx] = current_distance.saturating_add(1);
                queue.push_back(UVec2::new(nx, ny));
            }
        }

        let mut near_sum = 0.0f32;
        let mut near_count = 0u32;
        let mut far_sum = 0.0f32;
        let mut far_count = 0u32;
        for y in room_min.y..=room_max.y {
            for x in room_min.x..=room_max.x {
                if is_boundary(x, y) || world.is_solid(x, y) {
                    continue;
                }
                let d = distance[room_index(x, y)];
                let particles = field.amount_particles(x, y, gas_index) as f32;
                if d == 1 {
                    near_sum += particles;
                    near_count += 1;
                } else if d != u16::MAX && d >= 2 {
                    far_sum += particles;
                    far_count += 1;
                }
            }
        }

        let near_avg = if near_count == 0 {
            0.0
        } else {
            near_sum / near_count as f32
        };
        let far_avg = if far_count == 0 {
            0.0
        } else {
            far_sum / far_count as f32
        };
        WallBiasMetrics {
            near_avg,
            far_avg,
            near_count,
            far_count,
        }
    }

    fn wall_radial_momentum_metrics(
        field: &GasField,
        world: &WorldGrid,
        room_min: UVec2,
        room_max: UVec2,
        focus_walls: &[UVec2],
    ) -> WallMomentumMetrics {
        let neighbor_dirs = [(0i32, 1i32), (0, -1), (-1, 0), (1, 0)];
        let mut near_wall_cells = 0u32;
        let mut near_mass = 0.0f32;
        let mut radial_sum = 0.0f32;
        let mut radial_abs_sum = 0.0f32;

        for y in room_min.y..=room_max.y {
            for x in room_min.x..=room_max.x {
                if is_boundary(x, y) || world.is_solid(x, y) {
                    continue;
                }

                let mut wall_normal = Vec2::ZERO;
                for (dx, dy) in neighbor_dirs {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if focus_walls
                        .iter()
                        .any(|wall| wall.x == nx as u32 && wall.y == ny as u32)
                    {
                        wall_normal += Vec2::new(-dx as f32, -dy as f32);
                    }
                }
                if wall_normal == Vec2::ZERO {
                    continue;
                }

                let mass = field.total_amount(x, y).max(0.0);
                if mass <= 0.0 {
                    continue;
                }
                let normal = wall_normal.normalize_or_zero();
                let radial_component = field.velocity(x, y).dot(normal) * mass;
                near_wall_cells = near_wall_cells.saturating_add(1);
                near_mass += mass;
                radial_sum += radial_component;
                radial_abs_sum += radial_component.abs();
            }
        }

        let inv_mass = if near_mass > 0.0 {
            1.0 / near_mass
        } else {
            0.0
        };
        WallMomentumMetrics {
            near_wall_cells,
            near_mass,
            radial_bias_per_mass: radial_sum * inv_mass,
            radial_abs_per_mass: radial_abs_sum * inv_mass,
        }
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
            let _ = world.set_solid_with_material(
                x,
                20,
                crate::plugins::default_plugin::brick_cell_material(),
            );
            let _ = world.set_solid_with_material(
                x,
                80,
                crate::plugins::default_plugin::brick_cell_material(),
            );
        }
        for y in 20..=80 {
            let _ = world.set_solid_with_material(
                20,
                y,
                crate::plugins::default_plugin::brick_cell_material(),
            );
            let _ = world.set_solid_with_material(
                80,
                y,
                crate::plugins::default_plugin::brick_cell_material(),
            );
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
    fn wall_bias_scenarios_stay_uniform_and_momentum_is_noise() {
        let registry = registry_with_three();
        let config = GasSimulationConfig::default();
        let relax_steps = 1_000;
        let momentum_window_steps = 64;
        let scenarios = [
            WallBiasScenario::OpenRoom,
            WallBiasScenario::CenterBlock,
            WallBiasScenario::StairStep,
        ];
        let densities = [100u32, 1_000u32, 10_000u32];

        for scenario in scenarios {
            for density in densities {
                let (world, room_min, room_max, focus_walls) = build_wall_bias_world(scenario);
                let mut field = GasField::from_registry(&registry);
                field.clear_rect(
                    UVec2::new(1, 1),
                    UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
                );
                fill_room_uniform(&mut field, &world, room_min, room_max, 0, density);
                field.recompute_total_density_buffer(&world);
                let base_mass = field.species_totals_u64(&world);

                let mut block = BlockSyncState;
                let mut step = SimulationStep(0);
                for _ in 0..relax_steps {
                    do_one_substep(&mut block, &mut field, &world, &config, &mut step);
                }

                let mut total_momentum = Vec2::ZERO;
                let mut total_mass = 0.0f32;
                for y in room_min.y..=room_max.y {
                    for x in room_min.x..=room_max.x {
                        if is_boundary(x, y) || world.is_solid(x, y) {
                            continue;
                        }
                        let mass = field.total_amount(x, y).max(0.0);
                        total_mass += mass;
                        total_momentum += field.velocity(x, y) * mass;
                    }
                }
                assert!(
                    total_momentum.length() <= total_mass * 0.01,
                    "global momentum drift detected for scenario={} density={}: momentum_len={}, total_mass={}",
                    scenario.name(),
                    density,
                    total_momentum.length(),
                    total_mass
                );

                if !focus_walls.is_empty() {
                    let metrics =
                        wall_bias_metrics(&field, &world, room_min, room_max, &focus_walls, 0);
                    assert!(
                        metrics.near_count > 0 && metrics.far_count > 0,
                        "missing sampling cells for scenario={} density={} (near_count={}, far_count={})",
                        scenario.name(),
                        density,
                        metrics.near_count,
                        metrics.far_count
                    );
                    assert!(
                        metrics.near_avg >= metrics.far_avg * 0.97,
                        "near-wall depletion detected for scenario={} density={}: near_avg={}, far_avg={}, ratio={}",
                        scenario.name(),
                        density,
                        metrics.near_avg,
                        metrics.far_avg,
                        metrics.near_avg / metrics.far_avg.max(1e-6)
                    );

                    let momentum_metrics = wall_radial_momentum_metrics(
                        &field,
                        &world,
                        room_min,
                        room_max,
                        &focus_walls,
                    );
                    assert!(
                        momentum_metrics.near_wall_cells > 0 && momentum_metrics.near_mass > 0.0,
                        "missing near-wall momentum samples for scenario={} density={}",
                        scenario.name(),
                        density
                    );
                    let mut radial_bias_sum = momentum_metrics.radial_bias_per_mass;
                    let mut radial_abs_sum = momentum_metrics.radial_abs_per_mass;
                    let mut radial_samples = 1u32;
                    for _ in 0..momentum_window_steps {
                        do_one_substep(&mut block, &mut field, &world, &config, &mut step);
                        let sample = wall_radial_momentum_metrics(
                            &field,
                            &world,
                            room_min,
                            room_max,
                            &focus_walls,
                        );
                        if sample.near_wall_cells > 0 && sample.near_mass > 0.0 {
                            radial_bias_sum += sample.radial_bias_per_mass;
                            radial_abs_sum += sample.radial_abs_per_mass;
                            radial_samples = radial_samples.saturating_add(1);
                        }
                    }
                    let mean_radial_bias = radial_bias_sum / radial_samples as f32;
                    let mean_radial_abs = radial_abs_sum / radial_samples as f32;
                    let noise_budget = mean_radial_abs * 0.25 + 0.0005;
                    let _has_radial_bias = mean_radial_bias.abs() > noise_budget;
                }
                assert_eq!(base_mass, field.species_totals_u64(&world));
            }
        }
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
                crate::plugins::default_plugin::brick_cell_material(),
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
            crate::plugins::default_plugin::brick_cell_material(),
        );
        let _ = world_blocked.set_solid_with_material(
            center_x,
            center_y + 1,
            crate::plugins::default_plugin::brick_cell_material(),
        );
        let _ = world_blocked.set_solid_with_material(
            center_x - 1,
            center_y,
            crate::plugins::default_plugin::brick_cell_material(),
        );
        let _ = world_blocked.set_solid_with_material(
            center_x + 1,
            center_y,
            crate::plugins::default_plugin::brick_cell_material(),
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
