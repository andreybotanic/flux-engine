#[cfg(test)]
fn build_kernel_offsets(radius: i32) -> Vec<KernelOffset> {
    let mut offsets = Vec::new();
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx == 0 && dy == 0 {
                continue;
            }
            let dist2 = (dx * dx + dy * dy) as f32;
            offsets.push(KernelOffset { dx, dy, dist2 });
        }
    }
    offsets
}

#[cfg(test)]
fn kernel_offsets_for_radius(radius: u8) -> &'static [KernelOffset] {
    static KERNELS: OnceLock<Vec<Vec<KernelOffset>>> = OnceLock::new();
    let kernels = KERNELS.get_or_init(|| {
        vec![
            build_kernel_offsets(1),
            build_kernel_offsets(2),
            build_kernel_offsets(3),
        ]
    });
    let idx = radius.clamp(1, 3) as usize - 1;
    kernels[idx].as_slice()
}

#[cfg(test)]
fn estimate_local_env_mix_mass(
    field: &GasField,
    world: &WorldGrid,
    x: u32,
    y: u32,
    kernel: &[KernelOffset],
    kernel_weights: &[f32],
) -> Option<f32> {
    let mut weighted_mass_sum = 0.0f32;
    let mut weighted_rho_sum = 0.0f32;
    let reachable_samples_mask = kernel_sample_reachability_mask_von_neumann(world, x, y, kernel);

    for (sample_i, (sample, w)) in kernel
        .iter()
        .zip(kernel_weights.iter().copied())
        .enumerate()
    {
        if (reachable_samples_mask & (1u64 << sample_i)) == 0 {
            continue;
        }
        let nx = x as i32 + sample.dx;
        let ny = y as i32 + sample.dy;
        if nx < 0 || ny < 0 || nx >= WORLD_WIDTH as i32 || ny >= WORLD_HEIGHT as i32 {
            continue;
        }

        let nx = nx as u32;
        let ny = ny as u32;
        if is_boundary(nx, ny) || world.is_solid(nx, ny) {
            continue;
        }

        let nidx = linear_index(nx, ny);
        let rho_n: f32 = field.read[nidx].iter().map(|&v| v as f32).sum();
        if rho_n <= EPSILON {
            continue;
        }

        let mut m_mix_n = 0.0f32;
        for gas_index in 0..field.gas_count {
            let ci = field.read[nidx][gas_index] as f32;
            if ci <= EPSILON {
                continue;
            }
            m_mix_n += (ci / rho_n) * field.molecular_mass(gas_index);
        }
        if !m_mix_n.is_finite() || m_mix_n <= BUOYANCY_MIN_ENV_MASS {
            continue;
        }

        let wrho = w * rho_n;
        weighted_rho_sum += wrho;
        weighted_mass_sum += wrho * m_mix_n;
    }

    if weighted_rho_sum <= EPSILON {
        return None;
    }

    let m_env = weighted_mass_sum / weighted_rho_sum;
    if m_env.is_finite() && m_env > BUOYANCY_MIN_ENV_MASS {
        Some(m_env)
    } else {
        None
    }
}

#[cfg(test)]
fn kernel_sample_reachability_mask_von_neumann(
    world: &WorldGrid,
    x0: u32,
    y0: u32,
    kernel: &[KernelOffset],
) -> u64 {
    if kernel.is_empty() {
        return 0;
    }

    debug_assert!(kernel.len() <= 64);

    let mut min_dx = kernel[0].dx;
    let mut max_dx = kernel[0].dx;
    let mut min_dy = kernel[0].dy;
    let mut max_dy = kernel[0].dy;
    for sample in kernel.iter().copied() {
        min_dx = min_dx.min(sample.dx);
        max_dx = max_dx.max(sample.dx);
        min_dy = min_dy.min(sample.dy);
        max_dy = max_dy.max(sample.dy);
    }

    let width_i = max_dx - min_dx + 1;
    let height_i = max_dy - min_dy + 1;
    if width_i <= 0 || height_i <= 0 {
        return 0;
    }
    let width = width_i as usize;
    let height = height_i as usize;
    let area = width * height;
    if area == 0 || area > 64 {
        return 0;
    }

    let sx_i = -min_dx;
    let sy_i = -min_dy;
    if sx_i < 0 || sy_i < 0 {
        return 0;
    }
    let sx = sx_i as usize;
    let sy = sy_i as usize;
    if sx >= width || sy >= height {
        return 0;
    }

    let is_local_open = |lx: usize, ly: usize| -> bool {
        let wx = x0 as i32 + min_dx + lx as i32;
        let wy = y0 as i32 + min_dy + ly as i32;
        if wx < 0 || wy < 0 || wx >= WORLD_WIDTH as i32 || wy >= WORLD_HEIGHT as i32 {
            return false;
        }
        let wx = wx as u32;
        let wy = wy as u32;
        !is_boundary(wx, wy) && !world.is_solid(wx, wy)
    };

    if !is_local_open(sx, sy) {
        return 0;
    }

    let mut visited_local = 0u64;
    let mut queue = [0usize; 64];
    let mut head = 0usize;
    let mut tail = 0usize;

    let start_idx = sy * width + sx;
    visited_local |= 1u64 << start_idx;
    queue[tail] = start_idx;
    tail += 1;

    while head < tail {
        let current = queue[head];
        head += 1;
        let cx = current % width;
        let cy = current / width;

        if cy > 0 {
            let ny = cy - 1;
            let nidx = ny * width + cx;
            if (visited_local & (1u64 << nidx)) == 0 && is_local_open(cx, ny) {
                visited_local |= 1u64 << nidx;
                queue[tail] = nidx;
                tail += 1;
            }
        }
        if cy + 1 < height {
            let ny = cy + 1;
            let nidx = ny * width + cx;
            if (visited_local & (1u64 << nidx)) == 0 && is_local_open(cx, ny) {
                visited_local |= 1u64 << nidx;
                queue[tail] = nidx;
                tail += 1;
            }
        }
        if cx > 0 {
            let nx = cx - 1;
            let nidx = cy * width + nx;
            if (visited_local & (1u64 << nidx)) == 0 && is_local_open(nx, cy) {
                visited_local |= 1u64 << nidx;
                queue[tail] = nidx;
                tail += 1;
            }
        }
        if cx + 1 < width {
            let nx = cx + 1;
            let nidx = cy * width + nx;
            if (visited_local & (1u64 << nidx)) == 0 && is_local_open(nx, cy) {
                visited_local |= 1u64 << nidx;
                queue[tail] = nidx;
                tail += 1;
            }
        }
    }

    let mut sample_mask = 0u64;
    for (sample_i, sample) in kernel.iter().copied().enumerate() {
        let lx_i = sample.dx - min_dx;
        let ly_i = sample.dy - min_dy;
        if lx_i < 0 || ly_i < 0 {
            continue;
        }
        let lx = lx_i as usize;
        let ly = ly_i as usize;
        if lx >= width || ly >= height {
            continue;
        }
        let local_idx = ly * width + lx;
        if (visited_local & (1u64 << local_idx)) != 0 {
            sample_mask |= 1u64 << sample_i;
        }
    }

    sample_mask
}

