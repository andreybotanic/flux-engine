use bevy::prelude::*;
use std::sync::OnceLock;

use super::SolverTuning;

const EPSILON: f32 = 1e-6;
const BUOYANCY_MIN_ENV_MASS: f32 = 1e-6;
const DIRS_VON_NEUMANN: [IVec2; 4] = [
    IVec2::new(0, 1),
    IVec2::new(0, -1),
    IVec2::new(-1, 0),
    IVec2::new(1, 0),
];

#[derive(Clone, Copy)]
struct KernelOffset {
    dx: i32,
    dy: i32,
    dist2: f32,
}

#[derive(Clone, Copy)]
struct Rng64 {
    state: u32,
}

impl Rng64 {
    fn seeded(seed: u32) -> Self {
        Self {
            state: seed ^ 0x9E37_79B9,
        }
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    fn next_f32(&mut self) -> f32 {
        self.next_u32() as f32 / 4_294_967_296.0
    }
}

pub trait SolidQuery {
    fn is_solid(&self, x: u32, y: u32) -> bool;
}

impl<F> SolidQuery for F
where
    F: Fn(u32, u32) -> bool,
{
    fn is_solid(&self, x: u32, y: u32) -> bool {
        self(x, y)
    }
}

pub struct DiscreteStepParams<'a, S, T>
where
    S: SolidQuery,
    T: Fn(u32, u32) -> f32,
{
    pub width: u32,
    pub height: u32,
    pub gas_count: usize,
    pub molecular_masses: &'a [f32],
    pub read: &'a mut Vec<u32>,
    pub write: &'a mut Vec<u32>,
    pub total_density: &'a mut Vec<f32>,
    pub velocity: &'a mut Vec<Vec2>,
    pub solid_query: &'a S,
    pub temperature_multiplier: T,
    pub tuning: &'a SolverTuning,
    pub thermal_motion_scale: f32,
    pub simulation_step: u64,
}

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

                let mut weights = [0.0f32; 5];
                weights[4] = 1.0;
                let half_mobility = mobility * 0.5;

                // Direction up (0): blocked flow is redistributed along wall (left/right).
                if neighbor_open[0] {
                    weights[0] += mobility;
                } else if neighbor_open[2] && neighbor_open[3] {
                    weights[2] += half_mobility;
                    weights[3] += half_mobility;
                } else if neighbor_open[2] {
                    weights[2] += mobility;
                } else if neighbor_open[3] {
                    weights[3] += mobility;
                } else {
                    weights[4] += mobility;
                }

                // Direction down (1): blocked flow is redistributed along wall (left/right).
                if neighbor_open[1] {
                    weights[1] += mobility;
                } else if neighbor_open[2] && neighbor_open[3] {
                    weights[2] += half_mobility;
                    weights[3] += half_mobility;
                } else if neighbor_open[2] {
                    weights[2] += mobility;
                } else if neighbor_open[3] {
                    weights[3] += mobility;
                } else {
                    weights[4] += mobility;
                }

                // Direction left (2): blocked flow is redistributed along wall (up/down).
                if neighbor_open[2] {
                    weights[2] += mobility;
                } else if neighbor_open[0] && neighbor_open[1] {
                    weights[0] += half_mobility;
                    weights[1] += half_mobility;
                } else if neighbor_open[0] {
                    weights[0] += mobility;
                } else if neighbor_open[1] {
                    weights[1] += mobility;
                } else {
                    weights[4] += mobility;
                }

                // Direction right (3): blocked flow is redistributed along wall (up/down).
                if neighbor_open[3] {
                    weights[3] += mobility;
                } else if neighbor_open[0] && neighbor_open[1] {
                    weights[0] += half_mobility;
                    weights[1] += half_mobility;
                } else if neighbor_open[0] {
                    weights[0] += mobility;
                } else if neighbor_open[1] {
                    weights[1] += mobility;
                } else {
                    weights[4] += mobility;
                }

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

fn estimate_local_env_mix_mass<S: SolidQuery>(
    solid_query: &S,
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    gas_count: usize,
    read: &[u32],
    molecular_masses: &[f32],
    kernel: &[KernelOffset],
    kernel_weights: &[f32],
) -> Option<f32> {
    let mut weighted_mass_sum = 0.0f32;
    let mut weighted_rho_sum = 0.0f32;
    let reachable_samples_mask =
        kernel_sample_reachability_mask_von_neumann(solid_query, width, height, x, y, kernel);

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
        if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
            continue;
        }

        let nx = nx as u32;
        let ny = ny as u32;
        if is_boundary(width, height, nx, ny) || solid_query.is_solid(nx, ny) {
            continue;
        }

        let nidx = linear_index(width, nx, ny);
        let base = nidx * gas_count;
        let rho_n: f32 = read[base..base + gas_count].iter().map(|&v| v as f32).sum();
        if rho_n <= EPSILON {
            continue;
        }

        let mut m_mix_n = 0.0f32;
        for gas_index in 0..gas_count {
            let ci = read[base + gas_index] as f32;
            if ci <= EPSILON {
                continue;
            }
            let molecular_mass = molecular_masses.get(gas_index).copied().unwrap_or(1.0);
            m_mix_n += (ci / rho_n) * molecular_mass;
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

fn kernel_sample_reachability_mask_von_neumann<S: SolidQuery>(
    solid_query: &S,
    width: u32,
    height: u32,
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
    let width_local = width_i as usize;
    let height_local = height_i as usize;
    let area = width_local * height_local;
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
    if sx >= width_local || sy >= height_local {
        return 0;
    }

    let is_local_open = |lx: usize, ly: usize| -> bool {
        let wx = x0 as i32 + min_dx + lx as i32;
        let wy = y0 as i32 + min_dy + ly as i32;
        if wx < 0 || wy < 0 || wx >= width as i32 || wy >= height as i32 {
            return false;
        }
        let wx = wx as u32;
        let wy = wy as u32;
        !is_boundary(width, height, wx, wy) && !solid_query.is_solid(wx, wy)
    };

    if !is_local_open(sx, sy) {
        return 0;
    }

    let mut visited_local = 0u64;
    let mut queue = [0usize; 64];
    let mut head = 0usize;
    let mut tail = 0usize;

    let start_idx = sy * width_local + sx;
    visited_local |= 1u64 << start_idx;
    queue[tail] = start_idx;
    tail += 1;

    while head < tail {
        let current = queue[head];
        head += 1;
        let cx = current % width_local;
        let cy = current / width_local;

        if cy > 0 {
            let ny = cy - 1;
            let nidx = ny * width_local + cx;
            if (visited_local & (1u64 << nidx)) == 0 && is_local_open(cx, ny) {
                visited_local |= 1u64 << nidx;
                queue[tail] = nidx;
                tail += 1;
            }
        }
        if cy + 1 < height_local {
            let ny = cy + 1;
            let nidx = ny * width_local + cx;
            if (visited_local & (1u64 << nidx)) == 0 && is_local_open(cx, ny) {
                visited_local |= 1u64 << nidx;
                queue[tail] = nidx;
                tail += 1;
            }
        }
        if cx > 0 {
            let nx = cx - 1;
            let nidx = cy * width_local + nx;
            if (visited_local & (1u64 << nidx)) == 0 && is_local_open(nx, cy) {
                visited_local |= 1u64 << nidx;
                queue[tail] = nidx;
                tail += 1;
            }
        }
        if cx + 1 < width_local {
            let nx = cx + 1;
            let nidx = cy * width_local + nx;
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
        if lx >= width_local || ly >= height_local {
            continue;
        }
        let local_idx = ly * width_local + lx;
        if (visited_local & (1u64 << local_idx)) != 0 {
            sample_mask |= 1u64 << sample_i;
        }
    }

    sample_mask
}

fn split_count_by_weights(count: u32, weights: &[f32; 5], rng: &mut Rng64) -> [u32; 5] {
    let mut result = [0u32; 5];
    if count == 0 {
        return result;
    }

    let positive_total: f32 = weights
        .iter()
        .map(|w| w.max(0.0))
        .sum::<f32>()
        .max(f32::EPSILON);

    let mut fracs = [0.0f32; 5];
    let mut base_sum = 0u32;
    for i in 0..5 {
        let raw = (count as f32) * weights[i].max(0.0) / positive_total;
        let base = raw.floor() as u32;
        result[i] = base;
        fracs[i] = (raw - base as f32).max(0.0);
        base_sum = base_sum.saturating_add(base);
    }

    let mut remaining = count.saturating_sub(base_sum);
    while remaining > 0 {
        let frac_sum: f32 = fracs.iter().sum();
        let pick = if frac_sum > f32::EPSILON {
            weighted_pick(&fracs, frac_sum, rng)
        } else {
            let mut ws = [0.0f32; 5];
            for i in 0..5 {
                ws[i] = weights[i].max(0.0);
            }
            let ws_sum: f32 = ws.iter().sum::<f32>().max(f32::EPSILON);
            weighted_pick(&ws, ws_sum, rng)
        };
        result[pick] = result[pick].saturating_add(1);
        fracs[pick] = 0.0;
        remaining -= 1;
    }

    result
}

fn weighted_pick(weights: &[f32; 5], sum: f32, rng: &mut Rng64) -> usize {
    let mut t = rng.next_f32() * sum;
    let mut fallback = 4usize;
    for (i, w) in weights.iter().copied().enumerate() {
        if w > 0.0 {
            fallback = i;
        }
        if w > 0.0 && t < w {
            return i;
        }
        t -= w;
    }
    fallback
}

#[inline]
fn is_boundary(width: u32, height: u32, x: u32, y: u32) -> bool {
    x == 0 || y == 0 || x == width - 1 || y == height - 1
}

#[inline]
fn linear_index(width: u32, x: u32, y: u32) -> usize {
    (y * width + x) as usize
}
