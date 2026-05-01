use std::sync::OnceLock;

use bevy::prelude::*;

use super::{GasSimulationConfig, SolverTuning};

const LBM_DIRS: [IVec2; 9] = [
    IVec2::new(0, 0),
    IVec2::new(1, 0),
    IVec2::new(-1, 0),
    IVec2::new(0, 1),
    IVec2::new(0, -1),
    IVec2::new(1, 1),
    IVec2::new(-1, 1),
    IVec2::new(-1, -1),
    IVec2::new(1, -1),
];

const LBM_WEIGHTS: [f32; 9] = [
    4.0 / 9.0,
    1.0 / 9.0,
    1.0 / 9.0,
    1.0 / 9.0,
    1.0 / 9.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0,
];

const LBM_OPPOSITE: [usize; 9] = [0, 2, 1, 4, 3, 7, 8, 5, 6];
const EPSILON_DENSITY: f32 = 1e-6;
const LBM_VELOCITY_CLAMP: f32 = 0.95;
const BUOYANCY_MIN_ENV_MASS: f32 = 1e-6;
const SPECIES_RELATIVE_DRIFT_SCALE: f32 = 0.45;

#[derive(Clone, Copy)]
struct KernelOffset {
    dx: i32,
    dy: i32,
    dist2: f32,
}

#[derive(Clone, Debug)]
pub struct PerfGasState {
    pub width: u32,
    pub height: u32,
    pub species_read: Vec<[f32; 2]>,
    species_write: Vec<[f32; 2]>,
    pub lbm_read: Vec<[f32; 9]>,
    lbm_write: Vec<[f32; 9]>,
    pub total_density: Vec<f32>,
    pub velocity: Vec<Vec2>,
    pub solid_mask: Vec<u32>,
    pub step: u64,
}

impl PerfGasState {
    pub fn seeded(width: u32, height: u32) -> Self {
        let cells = (width * height) as usize;
        let mut state = Self {
            width,
            height,
            species_read: vec![[0.0; 2]; cells],
            species_write: vec![[0.0; 2]; cells],
            lbm_read: vec![[0.0; 9]; cells],
            lbm_write: vec![[0.0; 9]; cells],
            total_density: vec![0.0; cells],
            velocity: vec![Vec2::ZERO; cells],
            solid_mask: vec![0; cells],
            step: 0,
        };

        for y in 0..height {
            for x in 0..width {
                let idx = state.index(x, y);
                if state.is_boundary(x, y) {
                    state.solid_mask[idx] = 1;
                    continue;
                }
            }
        }

        let cx = width / 2;
        let cy = height / 2;
        let cidx = state.index(cx, cy);
        state.species_read[cidx][0] = 10_000.0;

        for y in 0..height {
            for x in 0..width {
                let idx = state.index(x, y);
                if state.solid_mask[idx] != 0 {
                    continue;
                }
                let total = state.species_read[idx][0] + state.species_read[idx][1];
                state.total_density[idx] = total;
                if total <= EPSILON_DENSITY {
                    continue;
                }
                let distribution = distribute_scalar_d2q9_runtime(total);
                state.lbm_read[idx] = distribution;
                state.lbm_write[idx] = distribution;
            }
        }

        state.species_write.copy_from_slice(&state.species_read);
        state
    }

    pub fn from_gpu_host_state(
        width: u32,
        height: u32,
        state: &super::gpu_solver::GpuSolverHostState,
    ) -> Result<Self, String> {
        let cells = (width as usize)
            .checked_mul(height as usize)
            .ok_or_else(|| "Cells count overflow".to_string())?;
        if state.species.len() != cells
            || state.lbm_flat.len() != cells * 9
            || state.total_density.len() != cells
            || state.velocity.len() != cells
            || state.solid_mask.len() != cells
        {
            return Err("GpuSolverHostState has invalid sizes for PerfGasState".to_string());
        }

        let mut lbm_read = vec![[0.0f32; 9]; cells];
        for (idx, dirs) in lbm_read.iter_mut().enumerate() {
            let base = idx * 9;
            dirs.copy_from_slice(&state.lbm_flat[base..base + 9]);
        }

        Ok(Self {
            width,
            height,
            species_read: state.species.clone(),
            species_write: state.species.clone(),
            lbm_write: lbm_read.clone(),
            lbm_read,
            total_density: state.total_density.clone(),
            velocity: state
                .velocity
                .iter()
                .map(|v| Vec2::new(v[0], v[1]))
                .collect::<Vec<_>>(),
            solid_mask: state.solid_mask.clone(),
            step: 0,
        })
    }

    pub fn index(&self, x: u32, y: u32) -> usize {
        (y * self.width + x) as usize
    }

    fn is_boundary(&self, x: u32, y: u32) -> bool {
        x == 0 || y == 0 || x == self.width - 1 || y == self.height - 1
    }

    fn is_outside(&self, x: i32, y: i32) -> bool {
        x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32
    }

    pub fn species_totals(&self) -> [f32; 2] {
        let mut totals = [0.0; 2];
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = self.index(x, y);
                if self.solid_mask[idx] != 0 {
                    continue;
                }
                totals[0] += self.species_read[idx][0];
                totals[1] += self.species_read[idx][1];
            }
        }
        totals
    }

    pub fn to_gpu_host_state(&self) -> super::gpu_solver::GpuSolverHostState {
        let mut lbm_flat = Vec::with_capacity(self.lbm_read.len() * 9);
        for dirs in &self.lbm_read {
            lbm_flat.extend_from_slice(dirs);
        }
        let velocity = self.velocity.iter().map(|v| [v.x, v.y]).collect::<Vec<_>>();
        super::gpu_solver::GpuSolverHostState {
            species: self.species_read.clone(),
            lbm_flat,
            total_density: self.total_density.clone(),
            velocity,
            solid_mask: self.solid_mask.clone(),
        }
    }

    pub fn apply_gpu_host_state(&mut self, state: &super::gpu_solver::GpuSolverHostState) {
        self.species_read.copy_from_slice(&state.species);
        self.species_write.copy_from_slice(&state.species);
        self.total_density.copy_from_slice(&state.total_density);
        for (dst, src) in self.velocity.iter_mut().zip(state.velocity.iter()) {
            *dst = Vec2::new(src[0], src[1]);
        }
        for (idx, dirs) in self.lbm_read.iter_mut().enumerate() {
            let base = idx * 9;
            dirs.copy_from_slice(&state.lbm_flat[base..base + 9]);
        }
        self.lbm_write.copy_from_slice(&self.lbm_read);
    }

    pub fn do_one_substep(&mut self, config: &GasSimulationConfig) {
        let target_species_totals = self.species_totals();

        self.step_lbm_unified(
            &config.solver_tuning,
            config.enable_species_relaxation,
            config.enable_lbm_velocity,
        );

        let current_species_totals = self.species_totals();
        let mass_error = max_relative_mass_error(target_species_totals, current_species_totals);
        let mass_fix_every = config.mass_fix_every_n_steps;
        let periodic_mass_fix =
            mass_fix_every > 0 && (self.step + 1) % u64::from(mass_fix_every) == 0;
        let event_mass_fix = mass_error > config.mass_fix_error_threshold.max(0.0);
        let did_mass_fix = periodic_mass_fix || event_mass_fix;
        if did_mass_fix {
            self.renormalize_species_mass(
                target_species_totals,
                config.mass_fix_min_residual.max(0.0),
            );
        }

        self.recompute_total_density_buffer();

        let reconcile_every = config.reconcile_every_n_steps;
        let periodic_reconcile =
            reconcile_every > 0 && (self.step + 1) % u64::from(reconcile_every) == 0;
        if config.enable_lbm_velocity && (periodic_reconcile || event_mass_fix) {
            self.reconcile_lbm_from_species();
        }

        self.step += 1;
    }

    pub fn step_lbm_unified(
        &mut self,
        tuning: &SolverTuning,
        enable_species_relaxation: bool,
        enable_lbm_velocity: bool,
    ) {
        for entry in &mut self.lbm_write {
            *entry = [0.0; 9];
        }
        for entry in &mut self.species_write {
            *entry = [0.0; 2];
        }

        let velocity_limit = tuning.target_cfl_like_limit.clamp(0.1, 0.98);
        let omega_plus = (1.0 / tuning.tau_even.max(0.55)).clamp(0.01, 1.99);
        let omega_minus = (1.0 / tuning.tau_odd.max(0.55)).clamp(0.01, 1.99);
        let buoyancy_radius = tuning.buoyancy_window_radius.clamp(1, 3);
        let buoyancy_sigma = tuning.buoyancy_window_sigma.clamp(0.5, 3.0);
        let buoyancy_kernel = kernel_offsets_for_radius(buoyancy_radius);
        let inv_two_sigma_sq = 1.0 / (2.0 * buoyancy_sigma * buoyancy_sigma);
        let buoyancy_kernel_weights: Vec<f32> = buoyancy_kernel
            .iter()
            .map(|sample| (-sample.dist2 * inv_two_sigma_sq).exp())
            .collect();

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = self.index(x, y);
                if self.solid_mask[idx] != 0 {
                    self.velocity[idx] = Vec2::ZERO;
                    self.species_write[idx] = [0.0; 2];
                    continue;
                }

                let (rho, mut forced_u) = macroscopic_from_distributions(&self.lbm_read[idx]);
                let mut local_species_buoyancy = [0.0f32; 2];
                let mut local_mix_buoyancy = 0.0f32;
                let mut has_local_buoyancy_context = false;
                if !enable_lbm_velocity {
                    forced_u = Vec2::ZERO;
                } else if tuning.enable_buoyancy && rho > EPSILON_DENSITY {
                    let species_total = self.species_read[idx].iter().copied().sum::<f32>();
                    if species_total > EPSILON_DENSITY {
                        let alpha = tuning.buoyancy_alpha.max(0.0);
                        let gain = tuning.buoyancy_gain.max(0.0);
                        if let Some(m_env) = estimate_local_env_mix_mass(
                            self,
                            x,
                            y,
                            buoyancy_kernel,
                            &buoyancy_kernel_weights,
                        ) {
                            has_local_buoyancy_context = true;
                            let mut m_cell = 0.0f32;
                            for kind in 0..2 {
                                let amount = self.species_read[idx][kind].max(0.0);
                                if amount <= EPSILON_DENSITY {
                                    continue;
                                }
                                let yi = amount / species_total;
                                m_cell += yi * molecular_mass(kind);
                                let xi = (m_env - molecular_mass(kind)) / m_env;
                                let bi = (gain * xi).tanh() * xi.abs().powf(alpha);
                                local_species_buoyancy[kind] = bi;
                            }
                            let x_mix = (m_env - m_cell) / m_env;
                            local_mix_buoyancy = (gain * x_mix).tanh() * x_mix.abs().powf(alpha);
                        }

                        let cap = tuning.buoyancy_force_cap.abs();
                        let force_y =
                            (tuning.buoyancy_strength * local_mix_buoyancy).clamp(-cap, cap);
                        forced_u.y += force_y;
                    }
                }
                let forced_u = forced_u.clamp_length_max(velocity_limit);
                let feq = equilibrium_distributions(rho, forced_u);

                let mut post = [0.0; 9];
                for i in 0..9 {
                    let j = LBM_OPPOSITE[i];
                    let fi = self.lbm_read[idx][i];
                    let fj = self.lbm_read[idx][j];
                    let feqi = feq[i];
                    let feqj = feq[j];
                    let f_plus = 0.5 * (fi + fj);
                    let f_minus = 0.5 * (fi - fj);
                    let feq_plus = 0.5 * (feqi + feqj);
                    let feq_minus = 0.5 * (feqi - feqj);
                    post[i] = (fi
                        - omega_plus * (f_plus - feq_plus)
                        - omega_minus * (f_minus - feq_minus))
                        .max(0.0);
                }

                let post_sum: f32 = post.iter().sum();
                let mut species_targets = [idx; 9];

                for i in 0..9 {
                    let dir = LBM_DIRS[i];
                    let nx = x as i32 + dir.x;
                    let ny = y as i32 + dir.y;

                    if self.is_outside(nx, ny) {
                        let opposite = LBM_OPPOSITE[i];
                        self.lbm_write[idx][opposite] += post[i];
                        species_targets[i] = idx;
                        continue;
                    }

                    let nidx = self.index(nx as u32, ny as u32);
                    if self.solid_mask[nidx] != 0 {
                        let opposite = LBM_OPPOSITE[i];
                        self.lbm_write[idx][opposite] += post[i];
                        species_targets[i] = idx;
                    } else {
                        self.lbm_write[nidx][i] += post[i];
                        species_targets[i] = nidx;
                    }
                }

                for kind in 0..2 {
                    let amount = self.species_read[idx][kind];
                    if amount <= EPSILON_DENSITY {
                        continue;
                    }

                    if !enable_species_relaxation || post_sum <= EPSILON_DENSITY {
                        self.species_write[idx][kind] += amount;
                        continue;
                    }

                    let mut biased_sum = 0.0f32;
                    let mut biased_weights = post;
                    if tuning.enable_buoyancy && has_local_buoyancy_context {
                        let relative_b = local_species_buoyancy[kind] - local_mix_buoyancy;
                        let drift =
                            (relative_b * tuning.buoyancy_strength * SPECIES_RELATIVE_DRIFT_SCALE)
                                .clamp(-0.35, 0.35);
                        for dir in 0..9 {
                            let dir_y = LBM_DIRS[dir].y as f32;
                            let multiplier = (1.0 + drift * dir_y).max(0.0);
                            biased_weights[dir] = post[dir] * multiplier;
                            biased_sum += biased_weights[dir];
                        }
                    } else {
                        biased_sum = post_sum;
                    }
                    if biased_sum <= EPSILON_DENSITY {
                        self.species_write[idx][kind] += amount;
                        continue;
                    }

                    for (dir, weight) in biased_weights.into_iter().enumerate() {
                        if weight <= EPSILON_DENSITY {
                            continue;
                        }
                        let target_idx = species_targets[dir];
                        let share = amount * (weight / biased_sum);
                        if share <= EPSILON_DENSITY {
                            continue;
                        }
                        self.species_write[target_idx][kind] += share;
                    }
                }
            }
        }

        std::mem::swap(&mut self.lbm_read, &mut self.lbm_write);
        std::mem::swap(&mut self.species_read, &mut self.species_write);

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = self.index(x, y);
                if self.solid_mask[idx] != 0 {
                    self.velocity[idx] = Vec2::ZERO;
                    self.total_density[idx] = 0.0;
                    continue;
                }
                let (rho, u) = macroscopic_from_distributions(&self.lbm_read[idx]);
                self.total_density[idx] = rho.max(0.0);
                self.velocity[idx] = u.clamp_length_max(velocity_limit);
            }
        }
    }

    pub fn recompute_total_density_buffer(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = self.index(x, y);
                if self.solid_mask[idx] != 0 {
                    self.total_density[idx] = 0.0;
                    continue;
                }
                self.total_density[idx] = self.species_read[idx].iter().copied().sum();
            }
        }
    }

    pub fn reconcile_lbm_from_species(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = self.index(x, y);
                if self.solid_mask[idx] != 0 {
                    self.lbm_read[idx] = [0.0; 9];
                    self.lbm_write[idx] = [0.0; 9];
                    self.total_density[idx] = 0.0;
                    self.velocity[idx] = Vec2::ZERO;
                    continue;
                }

                let species_total: f32 = self.species_read[idx].iter().copied().sum();
                if species_total <= EPSILON_DENSITY {
                    self.lbm_read[idx] = [0.0; 9];
                    self.lbm_write[idx] = [0.0; 9];
                    self.total_density[idx] = 0.0;
                    self.velocity[idx] = Vec2::ZERO;
                    continue;
                }

                let lbm_total: f32 = self.lbm_read[idx].iter().sum();
                if lbm_total <= EPSILON_DENSITY {
                    let feq = equilibrium_distributions(
                        species_total,
                        self.velocity[idx].clamp_length_max(LBM_VELOCITY_CLAMP),
                    );
                    self.lbm_read[idx] = feq;
                    self.lbm_write[idx] = feq;
                } else {
                    let scale = species_total / lbm_total;
                    for i in 0..9 {
                        self.lbm_read[idx][i] = (self.lbm_read[idx][i] * scale).max(0.0);
                    }
                    self.lbm_write[idx] = self.lbm_read[idx];
                }

                let (rho, u) = macroscopic_from_distributions(&self.lbm_read[idx]);
                self.total_density[idx] = rho.max(0.0);
                self.velocity[idx] = u.clamp_length_max(LBM_VELOCITY_CLAMP);
            }
        }
    }

    pub fn renormalize_species_mass(&mut self, target_totals: [f32; 2], min_residual: f32) {
        let current_totals = self.species_totals();
        let mut scales = [1.0; 2];
        for kind in 0..2 {
            if target_totals[kind] <= EPSILON_DENSITY || current_totals[kind] <= EPSILON_DENSITY {
                continue;
            }
            scales[kind] = target_totals[kind] / current_totals[kind];
        }

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = self.index(x, y);
                if self.solid_mask[idx] != 0 {
                    continue;
                }
                for kind in 0..2 {
                    self.species_read[idx][kind] =
                        (self.species_read[idx][kind] * scales[kind]).max(0.0);
                }
            }
        }

        let mut corrected_totals = [0.0; 2];
        let mut fallback_index = None;
        let mut fallback_total = -1.0f32;
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = self.index(x, y);
                if self.solid_mask[idx] != 0 {
                    continue;
                }
                let cell_total: f32 = self.species_read[idx].iter().copied().sum();
                if cell_total > fallback_total {
                    fallback_total = cell_total;
                    fallback_index = Some(idx);
                }
                corrected_totals[0] += self.species_read[idx][0];
                corrected_totals[1] += self.species_read[idx][1];
            }
        }

        let residual_threshold = min_residual.max(0.0);
        for kind in 0..2 {
            let residual = target_totals[kind] - corrected_totals[kind];
            if residual.abs() <= residual_threshold {
                continue;
            }

            let available = corrected_totals[kind];
            if available > EPSILON_DENSITY {
                let mut last_idx = None;
                let mut distributed = 0.0f32;
                for y in 0..self.height {
                    for x in 0..self.width {
                        let idx = self.index(x, y);
                        if self.solid_mask[idx] != 0 {
                            continue;
                        }
                        let amount = self.species_read[idx][kind];
                        if amount <= EPSILON_DENSITY {
                            continue;
                        }
                        let share = residual * (amount / available);
                        self.species_read[idx][kind] =
                            (self.species_read[idx][kind] + share).max(0.0);
                        distributed += share;
                        last_idx = Some(idx);
                    }
                }
                let remnant = residual - distributed;
                if remnant.abs() > residual_threshold {
                    if let Some(idx) = last_idx.or(fallback_index) {
                        self.species_read[idx][kind] =
                            (self.species_read[idx][kind] + remnant).max(0.0);
                    }
                }
            } else if let Some(idx) = fallback_index {
                self.species_read[idx][kind] = (self.species_read[idx][kind] + residual).max(0.0);
            }
        }

        let mut final_totals = [0.0; 2];
        let mut max_kind_index = [None; 2];
        let mut max_kind_value = [-1.0f32; 2];
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = self.index(x, y);
                if self.solid_mask[idx] != 0 {
                    continue;
                }
                for kind in 0..2 {
                    let amount = self.species_read[idx][kind];
                    final_totals[kind] += amount;
                    if amount > max_kind_value[kind] {
                        max_kind_value[kind] = amount;
                        max_kind_index[kind] = Some(idx);
                    }
                }
            }
        }

        for kind in 0..2 {
            let residual = target_totals[kind] - final_totals[kind];
            if residual.abs() <= EPSILON_DENSITY {
                continue;
            }
            if let Some(idx) = max_kind_index[kind].or(fallback_index) {
                self.species_read[idx][kind] = (self.species_read[idx][kind] + residual).max(0.0);
            }
        }

        self.species_write.copy_from_slice(&self.species_read);
    }
}

fn max_relative_mass_error(target_totals: [f32; 2], current_totals: [f32; 2]) -> f32 {
    let mut max_error = 0.0f32;
    for i in 0..target_totals.len() {
        let target = target_totals[i].max(0.0);
        let current = current_totals[i].max(0.0);
        let abs_error = (target - current).abs();
        let rel = if target > 1e-6 {
            abs_error / target
        } else {
            abs_error
        };
        max_error = max_error.max(rel);
    }
    max_error
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

fn estimate_local_env_mix_mass(
    field: &PerfGasState,
    x: u32,
    y: u32,
    kernel: &[KernelOffset],
    kernel_weights: &[f32],
) -> Option<f32> {
    let mut weighted_mass_sum = 0.0f32;
    let mut weighted_rho_sum = 0.0f32;

    for (sample, w) in kernel.iter().zip(kernel_weights.iter().copied()) {
        let nx = x as i32 + sample.dx;
        let ny = y as i32 + sample.dy;
        if field.is_outside(nx, ny) {
            continue;
        }

        let nidx = field.index(nx as u32, ny as u32);
        if field.solid_mask[nidx] != 0 {
            continue;
        }

        let rho_n: f32 = field.species_read[nidx].iter().copied().sum();
        if rho_n <= EPSILON_DENSITY {
            continue;
        }

        let mut m_mix_n = 0.0f32;
        for kind in 0..2 {
            let ci = field.species_read[nidx][kind].max(0.0);
            if ci <= EPSILON_DENSITY {
                continue;
            }
            m_mix_n += (ci / rho_n) * molecular_mass(kind);
        }
        if !m_mix_n.is_finite() || m_mix_n <= BUOYANCY_MIN_ENV_MASS {
            continue;
        }

        let wrho = w * rho_n;
        weighted_rho_sum += wrho;
        weighted_mass_sum += wrho * m_mix_n;
    }

    if weighted_rho_sum <= EPSILON_DENSITY {
        return None;
    }

    let m_env = weighted_mass_sum / weighted_rho_sum;
    if m_env.is_finite() && m_env > BUOYANCY_MIN_ENV_MASS {
        Some(m_env)
    } else {
        None
    }
}

fn molecular_mass(kind: usize) -> f32 {
    match kind {
        0 => 2.016,
        _ => 31.998,
    }
}

fn distribute_scalar_d2q9_runtime(amount: f32) -> [f32; 9] {
    if amount <= EPSILON_DENSITY {
        return [0.0; 9];
    }
    let mut result = [0.0; 9];
    for i in 0..9 {
        result[i] = amount * LBM_WEIGHTS[i];
    }
    result
}

fn equilibrium_distributions(rho: f32, velocity: Vec2) -> [f32; 9] {
    let ux = velocity.x;
    let uy = velocity.y;
    let u_sq = ux * ux + uy * uy;

    let mut feq = [0.0; 9];
    for i in 0..9 {
        let cx = LBM_DIRS[i].x as f32;
        let cy = LBM_DIRS[i].y as f32;
        let cu = cx * ux + cy * uy;
        feq[i] = LBM_WEIGHTS[i] * rho * (1.0 + 3.0 * cu + 4.5 * cu * cu - 1.5 * u_sq);
        if feq[i].is_sign_negative() {
            feq[i] = 0.0;
        }
    }

    feq
}

fn macroscopic_from_distributions(f: &[f32; 9]) -> (f32, Vec2) {
    let rho: f32 = f.iter().sum();
    if rho <= EPSILON_DENSITY {
        return (0.0, Vec2::ZERO);
    }

    let mut momentum = Vec2::ZERO;
    for i in 0..9 {
        let c = LBM_DIRS[i];
        momentum.x += f[i] * c.x as f32;
        momentum.y += f[i] * c.y as f32;
    }

    (rho, momentum / rho)
}
