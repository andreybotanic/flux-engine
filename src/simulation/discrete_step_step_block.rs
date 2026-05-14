/// Runs `step_discrete_in_place` logic.
pub fn step_discrete_in_place<S, T>(params: DiscreteStepParams<'_, S, T>)
where
    S: SolidQuery,
    T: Fn(u32, u32) -> f32,
{
    let width = params.width;
    let height = params.height;
    let gas_count = params.gas_count;
    let cell_count = (width * height) as usize;

    if width < 3 || height < 3 || gas_count == 0 {
        return;
    }

    debug_assert_eq!(params.read.len(), cell_count * gas_count);
    debug_assert_eq!(params.write.len(), cell_count * gas_count);
    debug_assert_eq!(params.total_density.len(), cell_count);
    debug_assert_eq!(params.velocity.len(), cell_count);

    for entry in params.write.chunks_exact_mut(gas_count) {
        entry.fill(0);
    }

    let mut net_momentum = vec![Vec2::ZERO; cell_count];
    let mobility_base = params.thermal_motion_scale.max(0.0);
    let buoyancy_radius = params.tuning.buoyancy_window_radius.clamp(1, 3);
    let buoyancy_sigma = params.tuning.buoyancy_window_sigma.clamp(0.5, 3.0);
    let buoyancy_kernel = kernel_offsets_for_radius(buoyancy_radius);
    let inv_two_sigma_sq = 1.0 / (2.0 * buoyancy_sigma * buoyancy_sigma);
    let buoyancy_kernel_weights: Vec<f32> = buoyancy_kernel
        .iter()
        .map(|sample| (-sample.dist2 * inv_two_sigma_sq).exp())
        .collect();

    for y in 0..height {
        for x in 0..width {
            if is_boundary(width, height, x, y) || params.solid_query.is_solid(x, y) {
                continue;
            }

            let idx = linear_index(width, x, y);
            let temp_mul = (params.temperature_multiplier)(x, y).max(0.0);
            let mobility = (mobility_base * temp_mul).max(0.0);

            let mut neighbor_idx = [idx; 4];
            let mut neighbor_open = [false; 4];
            for (dir_i, dir) in DIRS_VON_NEUMANN.iter().enumerate() {
                let nx = x as i32 + dir.x;
                let ny = y as i32 + dir.y;
                if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                    continue;
                }
                let nx = nx as u32;
                let ny = ny as u32;
                if is_boundary(width, height, nx, ny) || params.solid_query.is_solid(nx, ny) {
                    continue;
                }
                neighbor_open[dir_i] = true;
                neighbor_idx[dir_i] = linear_index(width, nx, ny);
            }

            let m_env = if params.tuning.enable_buoyancy {
                estimate_local_env_mix_mass(
                    params.solid_query,
                    width,
                    height,
                    x,
                    y,
                    gas_count,
                    params.read,
                    params.molecular_masses,
                    buoyancy_kernel,
                    &buoyancy_kernel_weights,
                )
            } else {
                None
            };

            let cell_base = idx * gas_count;
            for gas_index in 0..gas_count {
                let count = params.read[cell_base + gas_index];
                if count == 0 {
                    continue;
                }

                let seed = (params.simulation_step as u32).wrapping_mul(0xD6E8_FD50)
                    ^ (idx as u32).wrapping_mul(0x9E37_79B9)
                    ^ (gas_index as u32).wrapping_mul(0x85EB_CA77);
                let mut rng = Rng64::seeded(seed);

                let mut weights = [mobility, mobility, mobility, mobility, 1.0f32];

                if let Some(m_env) = m_env {
                    if m_env > BUOYANCY_MIN_ENV_MASS && mobility > 0.0 {
                        let alpha = params.tuning.buoyancy_alpha.max(0.0);
                        let gain = params.tuning.buoyancy_gain.max(0.0);
                        let molecular_mass = params
                            .molecular_masses
                            .get(gas_index)
                            .copied()
                            .unwrap_or(1.0);
                        let xi = (m_env - molecular_mass) / m_env;
                        let bi = (gain * xi).tanh() * xi.abs().powf(alpha);
                        let cap = params.tuning.buoyancy_force_cap.abs();
                        let bias =
                            (params.tuning.buoyancy_strength * bi).clamp(-cap, cap) * mobility;
                        if neighbor_open[0] {
                            weights[0] = (weights[0] + bias.max(0.0)).max(0.0);
                        }
                        if neighbor_open[1] {
                            weights[1] = (weights[1] + (-bias).max(0.0)).max(0.0);
                        }
                    }
                }

                for dir_i in 0..4 {
                    if !neighbor_open[dir_i] {
                        weights[dir_i] = 0.0;
                        weights[4] += mobility;
                    }
                }

                let shares = split_count_by_weights(count, &weights, &mut rng);
                for dir_i in 0..4 {
                    let moved = shares[dir_i];
                    if moved == 0 {
                        continue;
                    }
                    if !neighbor_open[dir_i] {
                        // With zero weights for blocked directions this path should be unreachable,
                        // but keep a safe fallback to preserve mass on any numerical edge case.
                        let write_index = cell_base + gas_index;
                        params.write[write_index] = params.write[write_index].saturating_add(moved);
                        continue;
                    }
                    let target = neighbor_idx[dir_i];
                    let target_index = target * gas_count + gas_index;
                    params.write[target_index] = params.write[target_index].saturating_add(moved);
                    let dir = DIRS_VON_NEUMANN[dir_i].as_vec2();
                    let impulse = dir * moved as f32;
                    net_momentum[idx] -= impulse;
                    net_momentum[target] += impulse;
                }
                let stayed = shares[4];
                if stayed > 0 {
                    let write_index = cell_base + gas_index;
                    params.write[write_index] = params.write[write_index].saturating_add(stayed);
                }
            }
        }
    }

    std::mem::swap(params.read, params.write);

    for y in 0..height {
        for x in 0..width {
            let idx = linear_index(width, x, y);
            if is_boundary(width, height, x, y) || params.solid_query.is_solid(x, y) {
                params.total_density[idx] = 0.0;
                params.velocity[idx] = Vec2::ZERO;
                continue;
            }

            let base = idx * gas_count;
            let mass: f32 = params.read[base..base + gas_count]
                .iter()
                .map(|&v| v as f32)
                .sum();
            params.total_density[idx] = mass;
            params.velocity[idx] = if mass > 0.0 {
                net_momentum[idx] / mass
            } else {
                Vec2::ZERO
            };
        }
    }
}
