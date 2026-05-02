use super::gpu_solver::GpuSolverHostState;
use super::SolverTuning;
use crate::config::GasRegistry;
use crate::world::grid::{is_boundary, linear_index, WorldGrid, WORLD_HEIGHT, WORLD_WIDTH};
use bevy::prelude::*;
use std::sync::OnceLock;

pub const HYDROGEN_GPU_STORAGE_MAX_PARTICLES: u32 = 20_000;

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

pub type GasScalar = f32;
pub type GasCell = Vec<GasScalar>;

#[derive(Clone, Copy)]
struct KernelOffset {
    dx: i32,
    dy: i32,
    dist2: f32,
}

#[derive(Resource, Clone)]
pub struct GasField {
    pub read: Vec<GasCell>,
    write: Vec<GasCell>,
    total_density: Vec<f32>,
    lbm_read: Vec<[f32; 9]>,
    lbm_write: Vec<[f32; 9]>,
    velocity: Vec<Vec2>,
    gas_count: usize,
    molecular_masses: Vec<f32>,
}

impl Default for GasField {
    fn default() -> Self {
        let gases = vec!["h2", "o2", "co2"];
        let masses = vec![2.016, 31.998, 44.009];
        Self::new_with_masses(&gases, &masses)
    }
}

impl GasField {
    pub fn from_registry(registry: &GasRegistry) -> Self {
        let ids = registry
            .all()
            .iter()
            .map(|g| g.id.as_str())
            .collect::<Vec<_>>();
        let masses = registry.molecular_masses();
        Self::new_with_masses(&ids, &masses)
    }

    fn new_with_masses(ids: &[&str], molecular_masses: &[f32]) -> Self {
        let cells = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
        let gas_count = ids.len();
        let read = vec![vec![0.0; gas_count]; cells];

        let mut field = Self {
            write: read.clone(),
            read,
            total_density: vec![0.0; cells],
            lbm_read: vec![[0.0; 9]; cells],
            lbm_write: vec![[0.0; 9]; cells],
            velocity: vec![Vec2::ZERO; cells],
            gas_count,
            molecular_masses: molecular_masses.to_vec(),
        };

        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                let index = linear_index(x, y);
                let total: f32 = field.read[index].iter().copied().sum();
                field.total_density[index] = total.max(0.0);
                if total <= EPSILON_DENSITY {
                    continue;
                }

                let distribution = distribute_scalar_d2q9_runtime(total);
                for (dir, amount) in distribution.into_iter().enumerate() {
                    field.lbm_read[index][dir] = amount;
                    field.lbm_write[index][dir] = amount;
                }
                field.velocity[index] = Vec2::ZERO;
            }
        }

        field
    }

    pub fn gas_count(&self) -> usize {
        self.gas_count
    }

    pub fn molecular_mass(&self, gas_index: usize) -> f32 {
        self.molecular_masses.get(gas_index).copied().unwrap_or(1.0)
    }

    pub fn amount(&self, x: u32, y: u32, gas_index: usize) -> GasScalar {
        self.read[linear_index(x, y)][gas_index]
    }

    pub fn amount_rounded(&self, x: u32, y: u32, gas_index: usize) -> u32 {
        self.amount(x, y, gas_index).max(0.0).round() as u32
    }

    pub fn total_amount(&self, x: u32, y: u32) -> GasScalar {
        self.read[linear_index(x, y)].iter().copied().sum()
    }

    pub fn total_amount_rounded(&self, x: u32, y: u32) -> u32 {
        self.total_amount(x, y).max(0.0).round() as u32
    }

    pub fn total_density(&self, x: u32, y: u32) -> f32 {
        self.total_density[linear_index(x, y)]
    }

    pub fn velocity(&self, x: u32, y: u32) -> Vec2 {
        self.velocity[linear_index(x, y)]
    }

    pub fn set_amount(&mut self, x: u32, y: u32, gas_index: usize, amount: GasScalar) {
        let index = linear_index(x, y);
        let clamped = amount.max(0.0);
        self.read[index][gas_index] = clamped;
        self.write[index][gas_index] = clamped;
    }

    pub fn clear_cell(&mut self, x: u32, y: u32) {
        let index = linear_index(x, y);
        for v in &mut self.read[index] {
            *v = 0.0;
        }
        for v in &mut self.write[index] {
            *v = 0.0;
        }
        self.total_density[index] = 0.0;
        self.lbm_read[index] = [0.0; 9];
        self.lbm_write[index] = [0.0; 9];
        self.velocity[index] = Vec2::ZERO;
    }

    pub fn clear_rect(&mut self, min: UVec2, max: UVec2) {
        for y in min.y..=max.y {
            for x in min.x..=max.x {
                for gas_index in 0..self.gas_count {
                    let amount = self.amount(x, y, gas_index);
                    if amount > EPSILON_DENSITY {
                        self.apply_species_delta_with_lbm(x, y, gas_index, -amount);
                    }
                }
            }
        }
    }

    pub fn apply_rect(
        &mut self,
        min: UVec2,
        max: UVec2,
        gas_index: usize,
        amount: u32,
        replace: bool,
        world: &WorldGrid,
    ) {
        for y in min.y..=max.y {
            for x in min.x..=max.x {
                if world.is_solid(x, y) || is_boundary(x, y) {
                    continue;
                }

                if replace {
                    let current = self.amount(x, y, gas_index);
                    let target = amount as f32;
                    self.apply_species_delta_with_lbm(x, y, gas_index, target - current);
                } else {
                    self.apply_species_delta_with_lbm(x, y, gas_index, amount as f32);
                }
            }
        }
    }

    pub fn apply_species_delta_with_lbm(
        &mut self,
        x: u32,
        y: u32,
        gas_index: usize,
        delta: GasScalar,
    ) -> GasScalar {
        if delta.abs() <= EPSILON_DENSITY {
            return 0.0;
        }

        let index = linear_index(x, y);
        let before = self.read[index][gas_index];
        let unclamped = before + delta;
        let clamped = unclamped.max(0.0);
        let applied = clamped - before;
        if applied.abs() <= EPSILON_DENSITY {
            return 0.0;
        }

        self.read[index][gas_index] = clamped;
        self.write[index][gas_index] = clamped;
        self.apply_total_lbm_delta_at_index(index, applied);
        applied
    }

    fn apply_total_lbm_delta_at_index(&mut self, index: usize, delta: GasScalar) {
        if delta.abs() <= EPSILON_DENSITY {
            return;
        }

        if delta > 0.0 {
            let distribution = distribute_scalar_d2q9_runtime(delta);
            for (dir, amount) in distribution.into_iter().enumerate() {
                self.lbm_read[index][dir] += amount;
            }
        } else {
            let current_total: f32 = self.lbm_read[index].iter().sum();
            if current_total <= EPSILON_DENSITY {
                self.lbm_read[index] = [0.0; 9];
            } else {
                let remove = -delta;
                let remove_clamped = remove.min(current_total);
                let keep_scale = (current_total - remove_clamped) / current_total;
                for value in &mut self.lbm_read[index] {
                    *value = (*value * keep_scale).max(0.0);
                }
            }
        }

        self.lbm_write[index] = self.lbm_read[index];
        let (rho, u) = macroscopic_from_distributions(&self.lbm_read[index]);
        self.total_density[index] = rho.max(0.0);
        self.velocity[index] = u.clamp_length_max(LBM_VELOCITY_CLAMP);
    }

    pub fn recompute_total_density_buffer(&mut self, world: &WorldGrid) {
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                let index = linear_index(x, y);
                if is_boundary(x, y) || world.is_solid(x, y) {
                    self.total_density[index] = 0.0;
                    continue;
                }
                self.total_density[index] = self.read[index].iter().copied().sum();
            }
        }
    }

    pub fn sync_lbm_from_total_density(&mut self, world: &WorldGrid) {
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                let index = linear_index(x, y);
                if is_boundary(x, y) || world.is_solid(x, y) {
                    self.lbm_read[index] = [0.0; 9];
                    self.lbm_write[index] = [0.0; 9];
                    self.velocity[index] = Vec2::ZERO;
                    continue;
                }

                let rho = self.total_density[index].max(0.0);
                let u = self.velocity[index].clamp_length_max(LBM_VELOCITY_CLAMP);
                let feq = equilibrium_distributions(rho, u);
                self.lbm_read[index] = feq;
                self.lbm_write[index] = feq;
            }
        }
    }

    pub fn reconcile_lbm_from_species(&mut self, world: &WorldGrid) {
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                let index = linear_index(x, y);
                if is_boundary(x, y) || world.is_solid(x, y) {
                    self.lbm_read[index] = [0.0; 9];
                    self.lbm_write[index] = [0.0; 9];
                    self.total_density[index] = 0.0;
                    self.velocity[index] = Vec2::ZERO;
                    continue;
                }

                let species_total: f32 = self.read[index].iter().copied().sum();
                if species_total <= EPSILON_DENSITY {
                    self.lbm_read[index] = [0.0; 9];
                    self.lbm_write[index] = [0.0; 9];
                    self.total_density[index] = 0.0;
                    self.velocity[index] = Vec2::ZERO;
                    continue;
                }

                let lbm_total: f32 = self.lbm_read[index].iter().sum();
                if lbm_total <= EPSILON_DENSITY {
                    let feq = equilibrium_distributions(
                        species_total,
                        self.velocity[index].clamp_length_max(LBM_VELOCITY_CLAMP),
                    );
                    self.lbm_read[index] = feq;
                    self.lbm_write[index] = feq;
                } else {
                    let scale = species_total / lbm_total;
                    for i in 0..9 {
                        self.lbm_read[index][i] = (self.lbm_read[index][i] * scale).max(0.0);
                    }
                    self.lbm_write[index] = self.lbm_read[index];
                }

                let (rho, u) = macroscopic_from_distributions(&self.lbm_read[index]);
                self.total_density[index] = rho.max(0.0);
                self.velocity[index] = u.clamp_length_max(LBM_VELOCITY_CLAMP);
            }
        }
    }

    pub fn step_lbm_unified(
        &mut self,
        world: &WorldGrid,
        tuning: &SolverTuning,
        enable_species_relaxation: bool,
        enable_lbm_velocity: bool,
    ) {
        for entry in &mut self.lbm_write {
            *entry = [0.0; 9];
        }
        for entry in &mut self.write {
            entry.fill(0.0);
        }

        let velocity_limit = tuning.target_cfl_like_limit.clamp(0.1, 0.98);
        let velocity_damping = tuning.velocity_damping.clamp(0.0, 0.95);
        let velocity_keep = 1.0 - velocity_damping;
        let species_eq_blend = tuning.species_eq_blend.clamp(0.0, 1.0);
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

        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                let index = linear_index(x, y);
                if is_boundary(x, y) || world.is_solid(x, y) {
                    self.velocity[index] = Vec2::ZERO;
                    self.write[index].fill(0.0);
                    continue;
                }

                let (rho, mut forced_u) = macroscopic_from_distributions(&self.lbm_read[index]);
                let mut local_species_buoyancy = vec![0.0f32; self.gas_count];
                let mut local_mix_buoyancy = 0.0f32;
                let mut has_local_buoyancy_context = false;
                if !enable_lbm_velocity {
                    forced_u = Vec2::ZERO;
                } else if tuning.enable_buoyancy && rho > EPSILON_DENSITY {
                    let species_total = self.read[index].iter().copied().sum::<f32>();
                    if species_total > EPSILON_DENSITY {
                        let alpha = tuning.buoyancy_alpha.max(0.0);
                        let gain = tuning.buoyancy_gain.max(0.0);
                        if let Some(m_env) = estimate_local_env_mix_mass(
                            self,
                            world,
                            x,
                            y,
                            buoyancy_kernel,
                            &buoyancy_kernel_weights,
                        ) {
                            has_local_buoyancy_context = true;
                            let mut m_cell = 0.0f32;
                            for gas_index in 0..self.gas_count {
                                let amount = self.read[index][gas_index].max(0.0);
                                if amount <= EPSILON_DENSITY {
                                    continue;
                                }
                                let yi = amount / species_total;
                                m_cell += yi * self.molecular_mass(gas_index);
                                let xi = (m_env - self.molecular_mass(gas_index)) / m_env;
                                let bi = (gain * xi).tanh() * xi.abs().powf(alpha);
                                local_species_buoyancy[gas_index] = bi;
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
                let forced_u = (forced_u * velocity_keep).clamp_length_max(velocity_limit);
                let feq = equilibrium_distributions(rho, forced_u);

                let mut post = [0.0; 9];
                let mut post_sum = 0.0f32;
                for i in 0..9 {
                    let j = LBM_OPPOSITE[i];
                    let fi = self.lbm_read[index][i];
                    let fj = self.lbm_read[index][j];
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
                    post_sum += post[i];
                }

                let mut transport_weights = post;
                let mut transport_sum = post_sum;
                if species_eq_blend > 0.0 {
                    transport_sum = 0.0;
                    for dir in 0..9 {
                        let blended =
                            post[dir] * (1.0 - species_eq_blend) + feq[dir].max(0.0) * species_eq_blend;
                        transport_weights[dir] = blended.max(0.0);
                        transport_sum += transport_weights[dir];
                    }
                }

                let mut species_targets = [index; 9];

                for i in 0..9 {
                    let dir = LBM_DIRS[i];
                    let nx = x as i32 + dir.x;
                    let ny = y as i32 + dir.y;

                    if nx < 0 || ny < 0 || nx >= WORLD_WIDTH as i32 || ny >= WORLD_HEIGHT as i32 {
                        let opposite = LBM_OPPOSITE[i];
                        self.lbm_write[index][opposite] += post[i];
                        species_targets[i] = index;
                        continue;
                    }

                    let nx = nx as u32;
                    let ny = ny as u32;
                    if is_boundary(nx, ny) || world.is_solid(nx, ny) {
                        let opposite = LBM_OPPOSITE[i];
                        self.lbm_write[index][opposite] += post[i];
                        species_targets[i] = index;
                    } else {
                        let neighbour_index = linear_index(nx, ny);
                        self.lbm_write[neighbour_index][i] += post[i];
                        species_targets[i] = neighbour_index;
                    }
                }

                for gas_index in 0..self.gas_count {
                    let amount = self.read[index][gas_index];
                    if amount <= EPSILON_DENSITY {
                        continue;
                    }

                    if !enable_species_relaxation || transport_sum <= EPSILON_DENSITY {
                        self.write[index][gas_index] += amount;
                        continue;
                    }

                    let mut biased_sum = 0.0f32;
                    let mut biased_weights = transport_weights;
                    if tuning.enable_buoyancy && has_local_buoyancy_context {
                        let relative_b = local_species_buoyancy[gas_index] - local_mix_buoyancy;
                        let drift =
                            (relative_b * tuning.buoyancy_strength * SPECIES_RELATIVE_DRIFT_SCALE)
                                .clamp(-0.35, 0.35);
                        for dir in 0..9 {
                            let dir_y = LBM_DIRS[dir].y as f32;
                            let multiplier = (1.0 + drift * dir_y).max(0.0);
                            biased_weights[dir] = transport_weights[dir] * multiplier;
                            biased_sum += biased_weights[dir];
                        }
                    } else {
                        biased_sum = transport_sum;
                    }
                    if biased_sum <= EPSILON_DENSITY {
                        self.write[index][gas_index] += amount;
                        continue;
                    }

                    let scale = amount / biased_sum;
                    for dir in 0..9 {
                        let share = biased_weights[dir].max(0.0) * scale;
                        if share <= 0.0 {
                            continue;
                        }
                        let target = species_targets[dir];
                        self.write[target][gas_index] += share;
                    }
                }
            }
        }

        std::mem::swap(&mut self.read, &mut self.write);

        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                let index = linear_index(x, y);
                if is_boundary(x, y) || world.is_solid(x, y) {
                    self.read[index].fill(0.0);
                    self.write[index].fill(0.0);
                    self.total_density[index] = 0.0;
                    self.velocity[index] = Vec2::ZERO;
                    continue;
                }

                let (rho, u) = macroscopic_from_distributions(&self.lbm_write[index]);
                self.total_density[index] = rho.max(0.0);
                self.velocity[index] = if enable_lbm_velocity {
                    (u * velocity_keep).clamp_length_max(velocity_limit)
                } else {
                    Vec2::ZERO
                };
            }
        }

        std::mem::swap(&mut self.lbm_read, &mut self.lbm_write);
    }

    pub fn species_totals(&self, world: &WorldGrid) -> Vec<GasScalar> {
        let mut totals = vec![0.0f32; self.gas_count];
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                if is_boundary(x, y) || world.is_solid(x, y) {
                    continue;
                }
                let index = linear_index(x, y);
                for (gas_index, total) in totals.iter_mut().enumerate() {
                    *total += self.read[index][gas_index];
                }
            }
        }
        totals
    }

    pub fn renormalize_species_mass(
        &mut self,
        world: &WorldGrid,
        target_totals: &[f32],
        min_residual: f32,
    ) {
        let current_totals = self.species_totals(world);
        let mut scales = vec![1.0f32; self.gas_count];
        for gas_index in 0..self.gas_count {
            let target = target_totals.get(gas_index).copied().unwrap_or(0.0).max(0.0);
            let current = current_totals.get(gas_index).copied().unwrap_or(0.0).max(0.0);
            if target <= EPSILON_DENSITY || current <= EPSILON_DENSITY {
                continue;
            }
            scales[gas_index] = target / current;
        }

        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                if is_boundary(x, y) || world.is_solid(x, y) {
                    continue;
                }
                let index = linear_index(x, y);
                for gas_index in 0..self.gas_count {
                    self.read[index][gas_index] =
                        (self.read[index][gas_index] * scales[gas_index]).max(0.0);
                }
            }
        }

        let mut corrected_totals = vec![0.0f32; self.gas_count];
        let residual_threshold = min_residual.max(0.0);
        let mut fallback_index = None;
        let mut fallback_total = -1.0f32;
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                if is_boundary(x, y) || world.is_solid(x, y) {
                    continue;
                }
                let index = linear_index(x, y);
                let cell_total: f32 = self.read[index].iter().copied().sum();
                if cell_total > fallback_total {
                    fallback_total = cell_total;
                    fallback_index = Some(index);
                }
                for gas_index in 0..self.gas_count {
                    corrected_totals[gas_index] += self.read[index][gas_index];
                }
            }
        }

        for gas_index in 0..self.gas_count {
            let target = target_totals.get(gas_index).copied().unwrap_or(0.0).max(0.0);
            let residual = target - corrected_totals[gas_index];
            if residual.abs() <= residual_threshold {
                continue;
            }

            let available = corrected_totals[gas_index];
            if available > EPSILON_DENSITY {
                let mut last_idx = None;
                let mut distributed = 0.0f32;
                for y in 0..WORLD_HEIGHT {
                    for x in 0..WORLD_WIDTH {
                        if is_boundary(x, y) || world.is_solid(x, y) {
                            continue;
                        }
                        let index = linear_index(x, y);
                        let amount = self.read[index][gas_index];
                        if amount <= EPSILON_DENSITY {
                            continue;
                        }
                        let share = residual * (amount / available);
                        self.read[index][gas_index] =
                            (self.read[index][gas_index] + share).max(0.0);
                        distributed += share;
                        last_idx = Some(index);
                    }
                }

                let remnant = residual - distributed;
                if remnant.abs() > residual_threshold {
                    if let Some(index) = last_idx.or(fallback_index) {
                        self.read[index][gas_index] =
                            (self.read[index][gas_index] + remnant).max(0.0);
                    }
                }
            } else if let Some(index) = fallback_index {
                self.read[index][gas_index] = (self.read[index][gas_index] + residual).max(0.0);
            }
        }

        let mut final_totals = vec![0.0f32; self.gas_count];
        let mut max_kind_index = vec![None; self.gas_count];
        let mut max_kind_value = vec![-1.0f32; self.gas_count];
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                if is_boundary(x, y) || world.is_solid(x, y) {
                    continue;
                }
                let index = linear_index(x, y);
                for gas_index in 0..self.gas_count {
                    let amount = self.read[index][gas_index];
                    final_totals[gas_index] += amount;
                    if amount > max_kind_value[gas_index] {
                        max_kind_value[gas_index] = amount;
                        max_kind_index[gas_index] = Some(index);
                    }
                }
            }
        }

        for gas_index in 0..self.gas_count {
            let target = target_totals.get(gas_index).copied().unwrap_or(0.0).max(0.0);
            let residual = target - final_totals[gas_index];
            if residual.abs() <= EPSILON_DENSITY {
                continue;
            }
            if let Some(index) = max_kind_index[gas_index].or(fallback_index) {
                self.read[index][gas_index] = (self.read[index][gas_index] + residual).max(0.0);
            }
        }

        self.write.clone_from(&self.read);
    }

    pub fn to_gpu_host_state(&self, world: &WorldGrid) -> GpuSolverHostState {
        let mut species = Vec::with_capacity((WORLD_WIDTH * WORLD_HEIGHT) as usize);
        let mut velocity = Vec::with_capacity((WORLD_WIDTH * WORLD_HEIGHT) as usize);
        let mut solid_mask = Vec::with_capacity((WORLD_WIDTH * WORLD_HEIGHT) as usize);
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                let idx = linear_index(x, y);
                let h2 = self.read[idx].first().copied().unwrap_or(0.0);
                let o2 = self.read[idx].get(1).copied().unwrap_or(0.0);
                let co2 = self.read[idx].get(2).copied().unwrap_or(0.0);
                species.push([h2, o2, co2, 0.0]);
                let v = self.velocity[idx];
                velocity.push([v.x, v.y]);
                let solid = if is_boundary(x, y) || world.is_solid(x, y) {
                    1u32
                } else {
                    0u32
                };
                solid_mask.push(solid);
            }
        }

        let mut lbm_flat = Vec::with_capacity((WORLD_WIDTH * WORLD_HEIGHT * 9) as usize);
        for dirs in &self.lbm_read {
            lbm_flat.extend_from_slice(dirs);
        }

        GpuSolverHostState {
            gas_count: self.gas_count.min(3) as u32,
            molecular_masses: [
                self.molecular_mass(0),
                self.molecular_mass(1),
                self.molecular_mass(2),
            ],
            species,
            lbm_flat,
            total_density: self.total_density.clone(),
            velocity,
            solid_mask,
        }
    }

    pub fn apply_gpu_host_state(&mut self, state: &GpuSolverHostState) {
        let cells = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
        if state.species.len() != cells
            || state.lbm_flat.len() != cells * 9
            || state.total_density.len() != cells
            || state.velocity.len() != cells
        {
            return;
        }

        for idx in 0..cells {
            if self.gas_count > 0 {
                self.read[idx][0] = state.species[idx][0];
                self.write[idx][0] = state.species[idx][0];
            }
            if self.gas_count > 1 {
                self.read[idx][1] = state.species[idx][1];
                self.write[idx][1] = state.species[idx][1];
            }
            if self.gas_count > 2 {
                self.read[idx][2] = state.species[idx][2];
                self.write[idx][2] = state.species[idx][2];
            }
            self.total_density[idx] = state.total_density[idx];
            self.velocity[idx] = Vec2::new(state.velocity[idx][0], state.velocity[idx][1]);
        }

        for idx in 0..cells {
            let base = idx * 9;
            for dir in 0..9 {
                let value = state.lbm_flat[base + dir];
                self.lbm_read[idx][dir] = value;
                self.lbm_write[idx][dir] = value;
            }
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

    for (sample, w) in kernel.iter().zip(kernel_weights.iter().copied()) {
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
        let rho_n: f32 = field.read[nidx].iter().copied().sum();
        if rho_n <= EPSILON_DENSITY {
            continue;
        }

        let mut m_mix_n = 0.0f32;
        for gas_index in 0..field.gas_count {
            let ci = field.read[nidx][gas_index].max(0.0);
            if ci <= EPSILON_DENSITY {
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

fn distribute_scalar_d2q9_runtime(amount: GasScalar) -> [GasScalar; 9] {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GasDefinition;
    use crate::simulation::{do_one_substep, BlockSyncState, GasSimulationConfig, SimulationStep};
    use crate::world::grid::CellMaterial;
    use crate::world::grid::WORLD_HEIGHT;
    use crate::world::grid::WORLD_WIDTH;

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
        assert!((field.total_amount(10, 10) - 23.0).abs() < 1e-6);
    }

    #[test]
    fn renormalization_does_not_inject_corner_and_matches_targets() {
        let registry = registry_with_three();
        let mut field = GasField::from_registry(&registry);
        let world = WorldGrid::default();

        field.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );
        field.set_amount(50, 50, 0, 100.0);
        field.set_amount(51, 50, 0, 40.0);
        field.set_amount(51, 51, 0, 15.0);
        field.set_amount(50, 51, 1, 10.0);
        field.set_amount(52, 51, 2, 5.0);

        let targets = vec![155.00013, 10.0, 5.0];
        field.renormalize_species_mass(&world, &targets, 0.0);

        assert!(
            field.amount(1, 1, 0) <= 1e-6,
            "corner cell should stay empty after residual correction"
        );

        let totals = field.species_totals(&world);
        for i in 0..targets.len() {
            assert!(
                (totals[i] - targets[i]).abs() < 1e-3,
                "species {} total mismatch: got {}, expected {}",
                i,
                totals[i],
                targets[i]
            );
        }
    }

    #[test]
    fn no_nan_inf_long_run_three_species() {
        let registry = registry_with_three();
        let world = WorldGrid::default();
        let mut field = GasField::from_registry(&registry);
        field.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );
        let cx = WORLD_WIDTH / 2;
        let cy = WORLD_HEIGHT / 2;
        let _ = field.apply_species_delta_with_lbm(cx, cy, 0, 7_000.0);
        let _ = field.apply_species_delta_with_lbm(cx + 1, cy, 1, 4_000.0);
        let _ = field.apply_species_delta_with_lbm(cx, cy + 1, 2, 3_000.0);

        let mut step = SimulationStep(0);
        let mut block_state = BlockSyncState;
        let cfg = GasSimulationConfig::default();
        for _ in 0..2_000 {
            do_one_substep(&mut block_state, &mut field, &world, &cfg, &mut step);
        }

        for y in 1..WORLD_HEIGHT - 1 {
            for x in 1..WORLD_WIDTH - 1 {
                for gas_index in 0..field.gas_count() {
                    let amount = field.amount(x, y, gas_index);
                    assert!(amount.is_finite());
                    assert!(amount >= -1e-4);
                }
                let v = field.velocity(x, y);
                assert!(v.x.is_finite() && v.y.is_finite());
            }
        }
    }

    #[test]
    fn buoyancy_uniform_mixed_three_species_near_zero() {
        let registry = registry_with_three();
        let mut field = GasField::from_registry(&registry);
        let world = WorldGrid::default();
        field.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );

        for y in 1..WORLD_HEIGHT - 1 {
            for x in 1..WORLD_WIDTH - 1 {
                field.set_amount(x, y, 0, 10.0);
                field.set_amount(x, y, 1, 10.0);
                field.set_amount(x, y, 2, 10.0);
            }
        }
        field.recompute_total_density_buffer(&world);
        field.sync_lbm_from_total_density(&world);

        let tuning = SolverTuning {
            enable_buoyancy: true,
            buoyancy_strength: 0.4,
            buoyancy_window_radius: 2,
            buoyancy_window_sigma: 1.2,
            buoyancy_gain: 2.2,
            buoyancy_alpha: 0.9,
            buoyancy_force_cap: 0.5,
            ..Default::default()
        };
        field.step_lbm_unified(&world, &tuning, true, true);

        let mut sum_abs_vy = 0.0f32;
        let mut count = 0u32;
        for y in 3..WORLD_HEIGHT - 3 {
            for x in 3..WORLD_WIDTH - 3 {
                let vy = field.velocity(x, y).y;
                sum_abs_vy += vy.abs();
                count += 1;
            }
        }
        let avg_abs_vy = sum_abs_vy / count as f32;
        assert!(
            avg_abs_vy < 1e-3,
            "average |vy| should be near zero for uniform mixed field, got {}",
            avg_abs_vy
        );
    }

    fn add_test_obstacles(world: &mut WorldGrid) {
        let rects = [
            (6, 8, 28, 22),
            (34, 10, 56, 24),
            (60, 34, 72, 66),
            (74, 54, 98, 98),
            (18, 52, 40, 60),
        ];
        for (x0, y0, x1, y1) in rects {
            for y in y0..=y1 {
                for x in x0..=x1 {
                    if x == x0 || x == x1 || y == y0 || y == y1 {
                        let _ = world.set_solid_with_material(x, y, CellMaterial::Brick);
                    }
                }
            }
        }
    }

    fn high_frequency_wave_score(field: &GasField, world: &WorldGrid, gas_index: usize) -> f32 {
        let mut acc = 0.0f32;
        let mut count = 0u32;
        for y in 2..WORLD_HEIGHT - 2 {
            for x in 2..WORLD_WIDTH - 2 {
                if world.is_solid(x, y) || is_boundary(x, y) {
                    continue;
                }
                if world.is_solid(x - 1, y)
                    || world.is_solid(x + 1, y)
                    || world.is_solid(x, y - 1)
                    || world.is_solid(x, y + 1)
                {
                    continue;
                }

                let c = field.amount(x, y, gas_index).max(0.0);
                let lap_like = c
                    - 0.25
                        * (field.amount(x - 1, y, gas_index).max(0.0)
                            + field.amount(x + 1, y, gas_index).max(0.0)
                            + field.amount(x, y - 1, gas_index).max(0.0)
                            + field.amount(x, y + 1, gas_index).max(0.0));
                acc += lap_like.abs();
                count += 1;
            }
        }
        if count == 0 { 0.0 } else { acc / count as f32 }
    }

    #[test]
    fn wave_noise_decays_in_obstacle_world() {
        let registry = registry_with_three();
        let mut world = WorldGrid::default();
        add_test_obstacles(&mut world);

        let mut field = GasField::from_registry(&registry);
        field.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );

        for y in 1..WORLD_HEIGHT - 1 {
            for x in 1..WORLD_WIDTH - 1 {
                if world.is_solid(x, y) || is_boundary(x, y) {
                    continue;
                }
                let dx = x as f32 - WORLD_WIDTH as f32 * 0.5;
                let dy = y as f32 - WORLD_HEIGHT as f32 * 0.52;
                let r2 = dx * dx + dy * dy;
                let base = (-(r2 / 520.0)).exp() * 220.0;
                if base > 0.02 {
                    field.set_amount(x, y, 0, base);
                }
            }
        }
        field.recompute_total_density_buffer(&world);
        field.sync_lbm_from_total_density(&world);

        let mut step = SimulationStep(0);
        let mut block_state = BlockSyncState;
        let cfg = GasSimulationConfig::default();

        let mut score_early = 0.0f32;
        for i in 0..1600 {
            do_one_substep(&mut block_state, &mut field, &world, &cfg, &mut step);
            if i == 200 {
                score_early = high_frequency_wave_score(&field, &world, 0);
            }
        }
        let score_late = high_frequency_wave_score(&field, &world, 0);

        assert!(
            score_late <= score_early * 0.65,
            "high-frequency wave score did not decay enough: early={}, late={}",
            score_early,
            score_late
        );
    }

    #[test]
    fn dormant_species_do_not_appear_from_h2_only_initial_state() {
        let registry = registry_with_three();
        let world = WorldGrid::default();
        let mut field = GasField::from_registry(&registry);
        field.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );

        let cx = WORLD_WIDTH / 2;
        let cy = WORLD_HEIGHT / 2;
        let _ = field.apply_species_delta_with_lbm(cx, cy, 0, 10_000.0);

        let mut step = SimulationStep(0);
        let mut block_state = BlockSyncState;
        let cfg = GasSimulationConfig::default();
        for _ in 0..500 {
            do_one_substep(&mut block_state, &mut field, &world, &cfg, &mut step);
        }

        let totals = field.species_totals(&world);
        assert!(
            totals.get(1).copied().unwrap_or(0.0) <= 1e-3,
            "O2 should stay near zero, got {}",
            totals.get(1).copied().unwrap_or(0.0)
        );
        assert!(
            totals.get(2).copied().unwrap_or(0.0) <= 1e-3,
            "CO2 should stay near zero, got {}",
            totals.get(2).copied().unwrap_or(0.0)
        );
    }
}
