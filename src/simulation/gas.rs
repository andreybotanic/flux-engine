use super::SolverTuning;
use crate::world::grid::{is_boundary, linear_index, WorldGrid, WORLD_HEIGHT, WORLD_WIDTH};
use bevy::prelude::*;
use std::sync::OnceLock;

pub const INITIAL_HYDROGEN_CENTER_PARTICLES: u32 = 10_000;
pub const INITIAL_OXYGEN_CENTER_PARTICLES: u32 = 0;
pub const HYDROGEN_GPU_STORAGE_MAX_PARTICLES: u32 = INITIAL_HYDROGEN_CENTER_PARTICLES * 2;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GasKind {
    #[default]
    Hydrogen,
    Oxygen,
}

impl GasKind {
    pub const ALL: [Self; 2] = [Self::Hydrogen, Self::Oxygen];

    pub fn index(self) -> usize {
        match self {
            Self::Hydrogen => 0,
            Self::Oxygen => 1,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Hydrogen => "Hydrogen",
            Self::Oxygen => "Oxygen",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Hydrogen => Self::Oxygen,
            Self::Oxygen => Self::Hydrogen,
        }
    }

    pub fn molecular_mass(self) -> f32 {
        match self {
            Self::Hydrogen => 2.016,
            Self::Oxygen => 31.998,
        }
    }
}

const GAS_KIND_COUNT: usize = GasKind::ALL.len();
pub type GasScalar = f32;
type GasCell = [GasScalar; GAS_KIND_COUNT];

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
}

impl Default for GasField {
    fn default() -> Self {
        let cells = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
        let mut read = vec![[0.0; GAS_KIND_COUNT]; cells];

        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                let index = linear_index(x, y);
                read[index][GasKind::Hydrogen.index()] = seeded_hydrogen_amount(x, y) as f32;
                read[index][GasKind::Oxygen.index()] = seeded_oxygen_amount(x, y) as f32;
            }
        }

        let mut field = Self {
            write: read.clone(),
            read,
            total_density: vec![0.0; cells],
            lbm_read: vec![[0.0; 9]; cells],
            lbm_write: vec![[0.0; 9]; cells],
            velocity: vec![Vec2::ZERO; cells],
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
}

impl GasField {
    pub fn amount(&self, x: u32, y: u32, kind: GasKind) -> GasScalar {
        self.read[linear_index(x, y)][kind.index()]
    }

    pub fn amount_rounded(&self, x: u32, y: u32, kind: GasKind) -> u32 {
        self.amount(x, y, kind).max(0.0).round() as u32
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

    pub fn set_amount(&mut self, x: u32, y: u32, kind: GasKind, amount: GasScalar) {
        let index = linear_index(x, y);
        let clamped = amount.max(0.0);
        self.read[index][kind.index()] = clamped;
        self.write[index][kind.index()] = clamped;
    }

    pub fn add_amount(&mut self, x: u32, y: u32, kind: GasKind, amount: GasScalar) {
        let current = self.amount(x, y, kind);
        self.set_amount(x, y, kind, (current + amount).max(0.0));
    }

    pub fn clear_amount(&mut self, x: u32, y: u32, kind: GasKind) {
        self.set_amount(x, y, kind, 0.0);
    }

    pub fn clear_cell(&mut self, x: u32, y: u32) {
        let index = linear_index(x, y);
        self.read[index] = [0.0; GAS_KIND_COUNT];
        self.write[index] = [0.0; GAS_KIND_COUNT];
        self.total_density[index] = 0.0;
        self.lbm_read[index] = [0.0; 9];
        self.lbm_write[index] = [0.0; 9];
        self.velocity[index] = Vec2::ZERO;
    }

    pub fn clear_rect(&mut self, min: UVec2, max: UVec2) {
        for y in min.y..=max.y {
            for x in min.x..=max.x {
                for kind in GasKind::ALL {
                    let amount = self.amount(x, y, kind);
                    if amount > EPSILON_DENSITY {
                        self.apply_species_delta_with_lbm(x, y, kind, -amount);
                    }
                }
            }
        }
    }

    pub fn apply_rect(
        &mut self,
        min: UVec2,
        max: UVec2,
        kind: GasKind,
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
                    let current = self.amount(x, y, kind);
                    let target = amount as f32;
                    self.apply_species_delta_with_lbm(x, y, kind, target - current);
                } else {
                    self.apply_species_delta_with_lbm(x, y, kind, amount as f32);
                }
            }
        }
    }

    pub fn apply_species_delta_with_lbm(
        &mut self,
        x: u32,
        y: u32,
        kind: GasKind,
        delta: GasScalar,
    ) -> GasScalar {
        if delta.abs() <= EPSILON_DENSITY {
            return 0.0;
        }

        let index = linear_index(x, y);
        let kind_index = kind.index();
        let before = self.read[index][kind_index];
        let unclamped = before + delta;
        let clamped = unclamped.max(0.0);
        let applied = clamped - before;
        if applied.abs() <= EPSILON_DENSITY {
            return 0.0;
        }

        self.read[index][kind_index] = clamped;
        self.write[index][kind_index] = clamped;
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
                let cell = self.read[index];
                self.total_density[index] = cell.iter().copied().sum();
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
            *entry = [0.0; GAS_KIND_COUNT];
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

        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                let index = linear_index(x, y);
                if is_boundary(x, y) || world.is_solid(x, y) {
                    self.velocity[index] = Vec2::ZERO;
                    self.write[index] = [0.0; GAS_KIND_COUNT];
                    continue;
                }

                let (rho, mut forced_u) = macroscopic_from_distributions(&self.lbm_read[index]);
                let mut local_species_buoyancy = [0.0f32; GAS_KIND_COUNT];
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
                            for kind in GasKind::ALL {
                                let amount = self.read[index][kind.index()].max(0.0);
                                if amount <= EPSILON_DENSITY {
                                    continue;
                                }
                                let yi = amount / species_total;
                                m_cell += yi * kind.molecular_mass();
                                let xi = (m_env - kind.molecular_mass()) / m_env;
                                let bi = (gain * xi).tanh() * xi.abs().powf(alpha);
                                local_species_buoyancy[kind.index()] = bi;
                            }
                            let x_mix = (m_env - m_cell) / m_env;
                            local_mix_buoyancy =
                                (gain * x_mix).tanh() * x_mix.abs().powf(alpha);
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
                // TRT decomposition into even/odd moments for opposite directions.
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
                }

                let post_sum: f32 = post.iter().sum();
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

                for kind in GasKind::ALL {
                    let kind_index = kind.index();
                    let amount = self.read[index][kind_index];
                    if amount <= EPSILON_DENSITY {
                        continue;
                    }

                    if !enable_species_relaxation {
                        self.write[index][kind_index] += amount;
                        continue;
                    }

                    if post_sum <= EPSILON_DENSITY {
                        self.write[index][kind_index] += amount;
                        continue;
                    }

                    let mut biased_sum = 0.0f32;
                    let mut biased_weights = post;
                    if tuning.enable_buoyancy && has_local_buoyancy_context {
                        let relative_b = local_species_buoyancy[kind_index] - local_mix_buoyancy;
                        let drift = (relative_b
                            * tuning.buoyancy_strength
                            * SPECIES_RELATIVE_DRIFT_SCALE)
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
                        self.write[index][kind_index] += amount;
                        continue;
                    }

                    for (dir, weight) in biased_weights.into_iter().enumerate() {
                        if weight <= EPSILON_DENSITY {
                            continue;
                        }
                        let target_index = species_targets[dir];
                        let share = amount * (weight / biased_sum);
                        if share <= EPSILON_DENSITY {
                            continue;
                        }
                        self.write[target_index][kind_index] += share;
                    }
                }
            }
        }

        std::mem::swap(&mut self.lbm_read, &mut self.lbm_write);
        std::mem::swap(&mut self.read, &mut self.write);

        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                let index = linear_index(x, y);
                if is_boundary(x, y) || world.is_solid(x, y) {
                    self.velocity[index] = Vec2::ZERO;
                    self.total_density[index] = 0.0;
                    continue;
                }
                let (rho, u) = macroscopic_from_distributions(&self.lbm_read[index]);
                self.total_density[index] = rho.max(0.0);
                self.velocity[index] = u.clamp_length_max(velocity_limit);
            }
        }
    }

    pub fn clear_velocity(&mut self) {
        for v in &mut self.velocity {
            *v = Vec2::ZERO;
        }
    }

    pub fn species_totals(&self, world: &WorldGrid) -> [GasScalar; GAS_KIND_COUNT] {
        let mut totals = [0.0; GAS_KIND_COUNT];
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                if is_boundary(x, y) || world.is_solid(x, y) {
                    continue;
                }
                let index = linear_index(x, y);
                for (kind_index, total) in totals.iter_mut().enumerate() {
                    *total += self.read[index][kind_index];
                }
            }
        }
        totals
    }

    pub fn renormalize_species_mass(
        &mut self,
        world: &WorldGrid,
        target_totals: [GasScalar; GAS_KIND_COUNT],
        min_residual: GasScalar,
    ) {
        let current_totals = self.species_totals(world);
        let mut scales = [1.0; GAS_KIND_COUNT];
        for kind_index in 0..GAS_KIND_COUNT {
            if target_totals[kind_index] <= EPSILON_DENSITY
                || current_totals[kind_index] <= EPSILON_DENSITY
            {
                continue;
            }
            scales[kind_index] = target_totals[kind_index] / current_totals[kind_index];
        }

        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                if is_boundary(x, y) || world.is_solid(x, y) {
                    continue;
                }
                let index = linear_index(x, y);
                for kind_index in 0..GAS_KIND_COUNT {
                    self.read[index][kind_index] =
                        (self.read[index][kind_index] * scales[kind_index]).max(0.0);
                }
            }
        }

        let mut corrected_totals = [0.0; GAS_KIND_COUNT];
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
                for (kind_index, total) in corrected_totals.iter_mut().enumerate() {
                    *total += self.read[index][kind_index];
                }
            }
        }

        let residual_threshold = min_residual.max(0.0);
        for kind_index in 0..GAS_KIND_COUNT {
            let residual = target_totals[kind_index] - corrected_totals[kind_index];
            if residual.abs() <= residual_threshold {
                continue;
            }

            let available = corrected_totals[kind_index];
            if available > EPSILON_DENSITY {
                let mut last_idx = None;
                let mut distributed = 0.0f32;
                for y in 0..WORLD_HEIGHT {
                    for x in 0..WORLD_WIDTH {
                        if is_boundary(x, y) || world.is_solid(x, y) {
                            continue;
                        }
                        let index = linear_index(x, y);
                        let amount = self.read[index][kind_index];
                        if amount <= EPSILON_DENSITY {
                            continue;
                        }

                        let share = residual * (amount / available);
                        self.read[index][kind_index] =
                            (self.read[index][kind_index] + share).max(0.0);
                        distributed += share;
                        last_idx = Some(index);
                    }
                }
                let remnant = residual - distributed;
                if remnant.abs() > residual_threshold {
                    if let Some(index) = last_idx.or(fallback_index) {
                        self.read[index][kind_index] =
                            (self.read[index][kind_index] + remnant).max(0.0);
                    }
                }
            } else if let Some(index) = fallback_index {
                self.read[index][kind_index] = (self.read[index][kind_index] + residual).max(0.0);
            }
        }

        // Final exactness pass: enforce target totals by assigning tiny remaining residual
        // to the densest cell of the corresponding species (never to a fixed corner anchor).
        let mut final_totals = [0.0; GAS_KIND_COUNT];
        let mut max_kind_index = [None; GAS_KIND_COUNT];
        let mut max_kind_value = [-1.0f32; GAS_KIND_COUNT];
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                if is_boundary(x, y) || world.is_solid(x, y) {
                    continue;
                }
                let index = linear_index(x, y);
                for kind_index in 0..GAS_KIND_COUNT {
                    let amount = self.read[index][kind_index];
                    final_totals[kind_index] += amount;
                    if amount > max_kind_value[kind_index] {
                        max_kind_value[kind_index] = amount;
                        max_kind_index[kind_index] = Some(index);
                    }
                }
            }
        }
        for kind_index in 0..GAS_KIND_COUNT {
            let residual = target_totals[kind_index] - final_totals[kind_index];
            if residual.abs() <= EPSILON_DENSITY {
                continue;
            }
            if let Some(index) = max_kind_index[kind_index].or(fallback_index) {
                self.read[index][kind_index] = (self.read[index][kind_index] + residual).max(0.0);
            }
        }

        self.write.copy_from_slice(&self.read);
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
        for kind in GasKind::ALL {
            let ci = field.read[nidx][kind.index()].max(0.0);
            if ci <= EPSILON_DENSITY {
                continue;
            }
            m_mix_n += (ci / rho_n) * kind.molecular_mass();
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

pub fn seeded_hydrogen_amount(x: u32, y: u32) -> u32 {
    if is_boundary(x, y) {
        return 0;
    }

    let center_x = WORLD_WIDTH / 2;
    let center_y = WORLD_HEIGHT / 2;

    if x == center_x && y == center_y {
        INITIAL_HYDROGEN_CENTER_PARTICLES
    } else {
        0
    }
}

pub fn seeded_oxygen_amount(x: u32, y: u32) -> u32 {
    if is_boundary(x, y) {
        return 0;
    }

    let center_x = WORLD_WIDTH / 2;
    let center_y = WORLD_HEIGHT / 2;

    if x == center_x && y == center_y {
        INITIAL_OXYGEN_CENTER_PARTICLES
    } else {
        0
    }
}

pub fn seeded_gas_amount(x: u32, y: u32) -> u32 {
    seeded_hydrogen_amount(x, y)
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
    use crate::world::grid::linear_index;

    #[test]
    fn seeded_center_hydrogen_and_zero_oxygen() {
        let center_x = WORLD_WIDTH / 2;
        let center_y = WORLD_HEIGHT / 2;

        assert_eq!(
            seeded_hydrogen_amount(center_x, center_y),
            INITIAL_HYDROGEN_CENTER_PARTICLES
        );
        assert_eq!(
            seeded_oxygen_amount(center_x, center_y),
            INITIAL_OXYGEN_CENTER_PARTICLES
        );
    }

    #[test]
    fn total_equals_sum_of_species_after_recompute() {
        let mut field = GasField::default();
        let world = WorldGrid::default();
        field.set_amount(10, 10, GasKind::Hydrogen, 17.0);
        field.set_amount(10, 10, GasKind::Oxygen, 23.0);

        field.recompute_total_density_buffer(&world);

        assert!((field.total_density(10, 10) - 40.0).abs() < 1e-6);
    }

    #[test]
    fn scalar_d2q9_distribution_preserves_mass() {
        let distributed = distribute_scalar_d2q9_runtime(360.0);
        let sum: f32 = distributed.iter().sum();
        assert!((sum - 360.0).abs() < 1e-4);
        assert!((distributed[0] - 160.0).abs() < 1e-4);
    }

    #[test]
    fn positive_delta_injects_zero_velocity_component_for_multiple_of_36() {
        let mut field = GasField::default();
        let x = 12;
        let y = 12;
        field.clear_cell(x, y);
        let index = linear_index(x, y);

        let applied = field.apply_species_delta_with_lbm(x, y, GasKind::Hydrogen, 360.0);
        assert!((applied - 360.0).abs() < 1e-6);

        let (rho, velocity) = macroscopic_from_distributions(&field.lbm_read[index]);
        assert_eq!(rho, 360.0);
        assert!(velocity.length() < 1e-6);
    }

    #[test]
    fn negative_delta_scales_lbm_without_flipping_velocity_direction() {
        let mut field = GasField::default();
        let x = 14;
        let y = 14;
        let index = linear_index(x, y);
        field.clear_cell(x, y);
        field.set_amount(x, y, GasKind::Hydrogen, 720.0);
        field.total_density[index] = 720.0;
        field.velocity[index] = Vec2::new(0.22, -0.11);
        let eq = equilibrium_distributions(720.0, field.velocity[index]);
        field.lbm_read[index] = eq;
        field.lbm_write[index] = eq;

        let before = macroscopic_from_distributions(&field.lbm_read[index]).1;
        let applied = field.apply_species_delta_with_lbm(x, y, GasKind::Hydrogen, -180.0);
        assert!((applied + 180.0).abs() < 1e-6);
        let after = macroscopic_from_distributions(&field.lbm_read[index]).1;

        assert!(after.x * before.x >= 0.0);
        assert!(after.y * before.y >= 0.0);
        assert!((after.x - before.x).abs() < 1e-4);
        assert!((after.y - before.y).abs() < 1e-4);
        assert!(field.lbm_read[index].iter().all(|v| *v >= -1e-6));
    }

    #[test]
    fn reconciliation_preserves_total_density_consistency() {
        let mut field = GasField::default();
        let world = WorldGrid::default();
        field.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );
        field.set_amount(20, 20, GasKind::Hydrogen, 123.25);
        field.set_amount(20, 20, GasKind::Oxygen, 11.75);
        field.recompute_total_density_buffer(&world);
        field.sync_lbm_from_total_density(&world);
        let total_species = field.total_amount(20, 20);
        assert!((field.total_density(20, 20) - total_species).abs() < 1e-5);
    }

    #[test]
    fn renormalization_does_not_inject_corner_when_residual_exists() {
        let mut field = GasField::default();
        let world = WorldGrid::default();
        field.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );
        field.set_amount(50, 50, GasKind::Hydrogen, 100.0);
        field.set_amount(51, 50, GasKind::Hydrogen, 40.0);
        field.set_amount(51, 51, GasKind::Hydrogen, 15.0);

        // Force a tiny residual correction path.
        field.renormalize_species_mass(&world, [155.00013, 0.0], 0.0);

        assert!(
            field.amount(1, 1, GasKind::Hydrogen) <= 1e-6,
            "Corner cell should stay empty after residual correction"
        );
        let total_h2: f32 = field
            .read
            .iter()
            .map(|cell| cell[GasKind::Hydrogen.index()])
            .sum();
        assert!((total_h2 - 155.00013).abs() < 1e-4);
    }

    #[test]
    fn buoyancy_local_env_finite_and_bounded() {
        let mut field = GasField::default();
        let world = WorldGrid::default();
        field.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );
        let x = WORLD_WIDTH / 2;
        let y = WORLD_HEIGHT / 2;
        field.set_amount(x, y, GasKind::Hydrogen, 1000.0);
        field.set_amount(x, y, GasKind::Oxygen, 1000.0);
        field.recompute_total_density_buffer(&world);
        field.sync_lbm_from_total_density(&world);

        let tuning = SolverTuning {
            enable_buoyancy: true,
            buoyancy_strength: 0.2,
            buoyancy_window_radius: 2,
            buoyancy_window_sigma: 1.2,
            buoyancy_gain: 2.0,
            buoyancy_alpha: 0.75,
            buoyancy_force_cap: 0.2,
            ..Default::default()
        };

        field.step_lbm_unified(&world, &tuning, true, true);

        for yy in 1..WORLD_HEIGHT - 1 {
            for xx in 1..WORLD_WIDTH - 1 {
                let v = field.velocity(xx, yy);
                assert!(v.x.is_finite() && v.y.is_finite());
                assert!(v.length() <= tuning.target_cfl_like_limit + 1e-5);
            }
        }
    }

    #[test]
    fn buoyancy_uniform_single_species_near_zero() {
        let mut field = GasField::default();
        let world = WorldGrid::default();
        field.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );

        for y in 1..WORLD_HEIGHT - 1 {
            for x in 1..WORLD_WIDTH - 1 {
                field.set_amount(x, y, GasKind::Hydrogen, 10.0);
            }
        }
        field.recompute_total_density_buffer(&world);
        field.sync_lbm_from_total_density(&world);

        let tuning = SolverTuning {
            enable_buoyancy: true,
            buoyancy_strength: 0.4,
            buoyancy_window_radius: 2,
            buoyancy_window_sigma: 1.2,
            buoyancy_gain: 2.0,
            buoyancy_alpha: 0.75,
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
            "Average |vy| should be near zero for uniform single-species field, got {}",
            avg_abs_vy
        );
    }

    #[test]
    fn buoyancy_uniform_mixed_species_near_zero() {
        let mut field = GasField::default();
        let world = WorldGrid::default();
        field.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );

        for y in 1..WORLD_HEIGHT - 1 {
            for x in 1..WORLD_WIDTH - 1 {
                field.set_amount(x, y, GasKind::Hydrogen, 10.0);
                field.set_amount(x, y, GasKind::Oxygen, 10.0);
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
            "Average |vy| should be near zero for uniform mixed field, got {}",
            avg_abs_vy
        );
    }

    #[test]
    fn buoyancy_light_vs_heavy_relative_sign() {
        let tuning = SolverTuning {
            enable_buoyancy: true,
            buoyancy_strength: 1.0,
            buoyancy_window_radius: 2,
            buoyancy_window_sigma: 1.2,
            buoyancy_gain: 2.0,
            buoyancy_alpha: 0.75,
            buoyancy_force_cap: 1.0,
            ..Default::default()
        };
        let m_env = 20.0f32;
        let h2_x = (m_env - GasKind::Hydrogen.molecular_mass()) / m_env;
        let o2_x = (m_env - GasKind::Oxygen.molecular_mass()) / m_env;
        let h2_b = (tuning.buoyancy_gain * h2_x).tanh() * h2_x.abs().powf(tuning.buoyancy_alpha);
        let o2_b = (tuning.buoyancy_gain * o2_x).tanh() * o2_x.abs().powf(tuning.buoyancy_alpha);
        assert!(h2_b > 0.0, "Hydrogen should be buoyant in this context");
        assert!(o2_b < 0.0, "Oxygen should sink in this context");
    }

    #[test]
    fn buoyancy_window_params_clamped() {
        let mut field = GasField::default();
        let world = WorldGrid::default();
        field.clear_rect(
            UVec2::new(1, 1),
            UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2),
        );
        let x = WORLD_WIDTH / 2;
        let y = WORLD_HEIGHT / 2;
        field.set_amount(x, y, GasKind::Hydrogen, 500.0);
        field.set_amount(x + 1, y, GasKind::Oxygen, 500.0);
        field.recompute_total_density_buffer(&world);
        field.sync_lbm_from_total_density(&world);

        let tuning = SolverTuning {
            enable_buoyancy: true,
            buoyancy_strength: 0.2,
            buoyancy_window_radius: 0,
            buoyancy_window_sigma: 0.01,
            buoyancy_gain: 2.0,
            buoyancy_alpha: 0.75,
            buoyancy_force_cap: 0.2,
            ..Default::default()
        };
        field.step_lbm_unified(&world, &tuning, true, true);

        for yy in 1..WORLD_HEIGHT - 1 {
            for xx in 1..WORLD_WIDTH - 1 {
                let v = field.velocity(xx, yy);
                assert!(v.x.is_finite() && v.y.is_finite());
            }
        }
    }
}
