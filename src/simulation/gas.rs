use bevy::prelude::*;
use crate::world::grid::{is_boundary, linear_index, WorldGrid, WORLD_HEIGHT, WORLD_WIDTH};

pub const HYDROGEN_DIFFUSION_K: f32 = 0.20;
pub const OXYGEN_DIFFUSION_K: f32 = 0.14;
pub const HYDROGEN_MAX_VISUAL_PARTICLES: u32 = 220;
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
const RANDOM_LCG_MULTIPLIER: u64 = 6364136223846793005;
const RANDOM_LCG_INCREMENT: u64 = 1442695040888963407;

const EPSILON_DENSITY: f32 = 1e-6;
const EPSILON_TRANSFER: f32 = 1e-7;
const ADVECTION_MAX_FRACTION: f32 = 0.45;
const ADVECTION_SPEED_SCALE: f32 = 0.55;
const LBM_VELOCITY_CLAMP: f32 = 0.95;

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

    pub fn short_label(self) -> &'static str {
        match self {
            Self::Hydrogen => "H2",
            Self::Oxygen => "O2",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Hydrogen => Self::Oxygen,
            Self::Oxygen => Self::Hydrogen,
        }
    }
}

const GAS_KIND_COUNT: usize = GasKind::ALL.len();
pub type GasScalar = f32;
type GasCell = [GasScalar; GAS_KIND_COUNT];

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
        self.read[linear_index(x, y)]
            .iter()
            .copied()
            .sum()
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

    pub fn step_lbm(&mut self, world: &WorldGrid, tau: f32) {
        for entry in &mut self.lbm_write {
            *entry = [0.0; 9];
        }
        for entry in &mut self.write {
            *entry = [0.0; GAS_KIND_COUNT];
        }

        let omega = (1.0 / tau.max(0.55)).min(1.99);

        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                let index = linear_index(x, y);
                if is_boundary(x, y) || world.is_solid(x, y) {
                    self.velocity[index] = Vec2::ZERO;
                    self.write[index] = [0.0; GAS_KIND_COUNT];
                    continue;
                }

                let (rho, u) = macroscopic_from_distributions(&self.lbm_read[index]);
                let feq = equilibrium_distributions(rho, u);

                let mut post = [0.0; 9];
                for i in 0..9 {
                    post[i] = (self.lbm_read[index][i] - omega * (self.lbm_read[index][i] - feq[i]))
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

                    if post_sum <= EPSILON_DENSITY {
                        self.write[index][kind_index] += amount;
                        continue;
                    }

                    for (dir, weight) in post.into_iter().enumerate() {
                        if weight <= EPSILON_DENSITY {
                            continue;
                        }
                        let target_index = species_targets[dir];
                        let share = amount * (weight / post_sum);
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
                self.velocity[index] = u.clamp_length_max(LBM_VELOCITY_CLAMP);
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
    ) {
        let current_totals = self.species_totals(world);
        let mut scales = [1.0; GAS_KIND_COUNT];
        for kind_index in 0..GAS_KIND_COUNT {
            if target_totals[kind_index] <= EPSILON_DENSITY || current_totals[kind_index] <= EPSILON_DENSITY {
                continue;
            }
            scales[kind_index] = target_totals[kind_index] / current_totals[kind_index];
        }

        let mut first_open_index = None;
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                if is_boundary(x, y) || world.is_solid(x, y) {
                    continue;
                }
                let index = linear_index(x, y);
                if first_open_index.is_none() {
                    first_open_index = Some(index);
                }
                for kind_index in 0..GAS_KIND_COUNT {
                    self.read[index][kind_index] = (self.read[index][kind_index] * scales[kind_index]).max(0.0);
                }
            }
        }

        if let Some(anchor_index) = first_open_index {
            let mut corrected_totals = [0.0; GAS_KIND_COUNT];
            for y in 0..WORLD_HEIGHT {
                for x in 0..WORLD_WIDTH {
                    if is_boundary(x, y) || world.is_solid(x, y) {
                        continue;
                    }
                    let index = linear_index(x, y);
                    for (kind_index, total) in corrected_totals.iter_mut().enumerate() {
                        *total += self.read[index][kind_index];
                    }
                }
            }

            for kind_index in 0..GAS_KIND_COUNT {
                let residual = target_totals[kind_index] - corrected_totals[kind_index];
                self.read[anchor_index][kind_index] =
                    (self.read[anchor_index][kind_index] + residual).max(0.0);
            }
        }

        self.write.copy_from_slice(&self.read);
    }

    pub fn advect_species_with_velocity(&mut self, world: &WorldGrid) {
        self.write.copy_from_slice(&self.read);

        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                let index = linear_index(x, y);
                if is_boundary(x, y) || world.is_solid(x, y) {
                    self.write[index] = [0.0; GAS_KIND_COUNT];
                    continue;
                }

                let velocity = self.velocity[index];
                let mut targets: [(Option<(u32, u32)>, f32); 4] = [(None, 0.0), (None, 0.0), (None, 0.0), (None, 0.0)];
                let mut target_count = 0usize;

                if velocity.x > 0.0 {
                    if let Some((tx, ty)) = open_neighbour_in_dir(world, x, y, IVec2::new(1, 0)) {
                        targets[target_count] = (Some((tx, ty)), velocity.x.abs());
                        target_count += 1;
                    }
                } else if velocity.x < 0.0 {
                    if let Some((tx, ty)) = open_neighbour_in_dir(world, x, y, IVec2::new(-1, 0)) {
                        targets[target_count] = (Some((tx, ty)), velocity.x.abs());
                        target_count += 1;
                    }
                }

                if velocity.y > 0.0 {
                    if let Some((tx, ty)) = open_neighbour_in_dir(world, x, y, IVec2::new(0, 1)) {
                        targets[target_count] = (Some((tx, ty)), velocity.y.abs());
                        target_count += 1;
                    }
                } else if velocity.y < 0.0 {
                    if let Some((tx, ty)) = open_neighbour_in_dir(world, x, y, IVec2::new(0, -1)) {
                        targets[target_count] = (Some((tx, ty)), velocity.y.abs());
                        target_count += 1;
                    }
                }

                if target_count == 0 {
                    continue;
                }

                let speed = velocity.length().min(1.0);
                if speed <= 0.0 {
                    continue;
                }

                let move_fraction = (speed * ADVECTION_SPEED_SCALE).min(ADVECTION_MAX_FRACTION);
                let total_weight: f32 = targets[..target_count].iter().map(|(_, w)| *w).sum();
                if total_weight <= 0.0 {
                    continue;
                }

                for kind in GasKind::ALL {
                    let kind_index = kind.index();
                    let amount = self.read[index][kind_index];
                    if amount <= EPSILON_DENSITY {
                        continue;
                    }

                    let total_move = amount * move_fraction;
                    if total_move <= EPSILON_TRANSFER {
                        continue;
                    }

                    let mut moved_sum = 0.0f32;
                    for (slot, (target, weight)) in targets[..target_count].iter().enumerate() {
                        let Some((tx, ty)) = *target else {
                            continue;
                        };
                        let neighbour_index = linear_index(tx, ty);

                        let share = if slot + 1 == target_count {
                            (total_move - moved_sum).max(0.0)
                        } else {
                            (total_move * (*weight / total_weight)).min((total_move - moved_sum).max(0.0))
                        };

                        if share <= EPSILON_TRANSFER {
                            continue;
                        }

                        self.write[index][kind_index] = (self.write[index][kind_index] - share).max(0.0);
                        self.write[neighbour_index][kind_index] += share;
                        moved_sum += share;
                    }
                }
            }
        }

        std::mem::swap(&mut self.read, &mut self.write);
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

pub fn step_cpu_gas_block_sync(
    gas: &mut GasField,
    world: &WorldGrid,
    block_index: u8,
    offset_x: u32,
    offset_y: u32,
    rng_state: &mut u64,
    diffusion_k: [f32; GAS_KIND_COUNT],
    max_flux_fraction: f32,
) {
    gas.write.copy_from_slice(&gas.read);

    let target_x = (block_index % 3) as u32;
    let target_y = (block_index / 3) as u32;

    for y in 0..WORLD_HEIGHT {
        for x in 0..WORLD_WIDTH {
            let index = linear_index(x, y);
            if is_boundary(x, y) || world.is_solid(x, y) {
                gas.write[index] = [0.0; GAS_KIND_COUNT];
                continue;
            }

            if !is_active_block_cell(x, y, target_x, target_y, offset_x, offset_y) {
                continue;
            }

            let neighbours = open_cardinal_neighbours(world, x, y);
            if neighbours.is_empty() {
                continue;
            }

            let pick = (next_random_u32(rng_state) as usize) % neighbours.len();
            let (nx, ny) = neighbours[pick];
            let neighbour_index = linear_index(nx, ny);

            for kind in GasKind::ALL {
                let kind_index = kind.index();
                let k = diffusion_k[kind_index].max(0.0);
                if k <= 0.0 {
                    continue;
                }

                let center = gas.read[index][kind_index].max(0.0);
                let neighbour = gas.read[neighbour_index][kind_index].max(0.0);
                let raw_flux = k * (center - neighbour);
                if raw_flux.abs() <= EPSILON_TRANSFER {
                    continue;
                }

                let limiter = max_flux_fraction.clamp(0.0, 1.0);
                let available_center = gas.write[index][kind_index].max(0.0);
                let available_neighbour = gas.write[neighbour_index][kind_index].max(0.0);
                let limited_max_from_center = (limiter * center).min(available_center);
                let limited_max_from_neighbour = (limiter * neighbour).min(available_neighbour);
                let flux = raw_flux
                    .max(-limited_max_from_neighbour)
                    .min(limited_max_from_center);
                if flux.abs() <= EPSILON_TRANSFER {
                    continue;
                }

                gas.write[index][kind_index] -= flux;
                gas.write[neighbour_index][kind_index] += flux;
            }
        }
    }

    std::mem::swap(&mut gas.read, &mut gas.write);
}

pub fn next_random_u32(state: &mut u64) -> u32 {
    *state = state
        .wrapping_mul(RANDOM_LCG_MULTIPLIER)
        .wrapping_add(RANDOM_LCG_INCREMENT);
    (*state >> 32) as u32
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

fn cardinal_neighbours(x: u32, y: u32) -> [(u32, u32); 4] {
    [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
}

fn open_cardinal_neighbours(world: &WorldGrid, x: u32, y: u32) -> Vec<(u32, u32)> {
    let mut result = Vec::with_capacity(4);
    for (nx, ny) in cardinal_neighbours(x, y) {
        if !is_boundary(nx, ny) && !world.is_solid(nx, ny) {
            result.push((nx, ny));
        }
    }
    result
}

fn open_neighbour_in_dir(world: &WorldGrid, x: u32, y: u32, dir: IVec2) -> Option<(u32, u32)> {
    let nx = x as i32 + dir.x;
    let ny = y as i32 + dir.y;
    if nx < 0 || ny < 0 || nx >= WORLD_WIDTH as i32 || ny >= WORLD_HEIGHT as i32 {
        return None;
    }

    let nx = nx as u32;
    let ny = ny as u32;
    if is_boundary(nx, ny) || world.is_solid(nx, ny) {
        None
    } else {
        Some((nx, ny))
    }
}

fn is_active_block_cell(
    x: u32,
    y: u32,
    target_x: u32,
    target_y: u32,
    offset_x: u32,
    offset_y: u32,
) -> bool {
    let local_x = (x + 3 - (offset_x % 3)) % 3;
    let local_y = (y + 3 - (offset_y % 3)) % 3;
    local_x == target_x && local_y == target_y
}

pub fn phase_offsets(phase: u8) -> (u32, u32) {
    let p = u32::from(phase % 9);
    (p % 3, (p / 3) % 3)
}

/// Preview what the next diffusion substep would do without modifying the gas field.
/// Returns `(block_index, moves)` where each move is `(from_x, from_y, to_x, to_y)`.
pub fn preview_next_substep(
    gas: &GasField,
    world: &WorldGrid,
    mut rng_state: u64,
    phase: u8,
    diffusion_k: [f32; GAS_KIND_COUNT],
    max_flux_fraction: f32,
) -> (u8, Vec<(u32, u32, u32, u32)>) {
    let block_index = (next_random_u32(&mut rng_state) % 9) as u8;
    let (offset_x, offset_y) = phase_offsets(phase);
    let target_x = (block_index % 3) as u32;
    let target_y = (block_index / 3) as u32;

    let mut moves = Vec::new();
    for y in 0..WORLD_HEIGHT {
        for x in 0..WORLD_WIDTH {
            if is_boundary(x, y) || world.is_solid(x, y) {
                continue;
            }
            if !is_active_block_cell(x, y, target_x, target_y, offset_x, offset_y) {
                continue;
            }

            let neighbours = open_cardinal_neighbours(world, x, y);
            if neighbours.is_empty() {
                continue;
            }

            let pick = (next_random_u32(&mut rng_state) as usize) % neighbours.len();
            let (nx, ny) = neighbours[pick];

            let has_movable = GasKind::ALL.into_iter().any(|kind| {
                let kind_index = kind.index();
                let k = diffusion_k[kind_index].max(0.0);
                if k <= 0.0 {
                    return false;
                }
                let center = gas.amount(x, y, kind).max(0.0);
                let neighbour = gas.amount(nx, ny, kind).max(0.0);
                let raw_flux = k * (center - neighbour);
                if raw_flux.abs() <= EPSILON_TRANSFER {
                    return false;
                }
                let limited_max_from_center = max_flux_fraction.clamp(0.0, 1.0) * center;
                let limited_max_from_neighbour = max_flux_fraction.clamp(0.0, 1.0) * neighbour;
                let flux = raw_flux
                    .max(-limited_max_from_neighbour)
                    .min(limited_max_from_center);
                flux.abs() > EPSILON_TRANSFER
            });
            if !has_movable {
                continue;
            }

            moves.push((x, y, nx, ny));
        }
    }

    (block_index, moves)
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

    fn total_species(field: &GasField, kind: GasKind) -> f32 {
        field.read.iter().map(|cell| cell[kind.index()]).sum()
    }

    #[test]
    fn seeded_center_hydrogen_and_zero_oxygen() {
        let center_x = WORLD_WIDTH / 2;
        let center_y = WORLD_HEIGHT / 2;

        assert_eq!(seeded_hydrogen_amount(center_x, center_y), INITIAL_HYDROGEN_CENTER_PARTICLES);
        assert_eq!(seeded_oxygen_amount(center_x, center_y), INITIAL_OXYGEN_CENTER_PARTICLES);
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
    fn diffusion_preserves_mass_and_non_negative() {
        let mut field = GasField::default();
        let world = WorldGrid::default();

        field.clear_rect(UVec2::new(1, 1), UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2));
        field.set_amount(4, 4, GasKind::Hydrogen, 100.0);
        field.set_amount(4, 4, GasKind::Oxygen, 100.0);

        let h_before = total_species(&field, GasKind::Hydrogen);
        let o_before = total_species(&field, GasKind::Oxygen);

        let mut seed = 1234;
        step_cpu_gas_block_sync(
            &mut field,
            &world,
            8,
            1,
            1,
            &mut seed,
            [HYDROGEN_DIFFUSION_K, OXYGEN_DIFFUSION_K],
            0.30,
        );

        assert!((h_before - total_species(&field, GasKind::Hydrogen)).abs() < 1e-5);
        assert!((o_before - total_species(&field, GasKind::Oxygen)).abs() < 1e-5);
        assert!(field
            .read
            .iter()
            .flat_map(|cell| cell.iter())
            .all(|value| *value >= -1e-6));
    }

    #[test]
    fn solids_block_all_species_diffusion() {
        let mut field = GasField::default();
        let mut world = WorldGrid::default();

        field.clear_rect(UVec2::new(1, 1), UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2));
        field.set_amount(1, 1, GasKind::Hydrogen, 50.0);
        field.set_amount(1, 1, GasKind::Oxygen, 50.0);
        assert!(world.set_solid(2, 1));

        let mut seed = 5;
        step_cpu_gas_block_sync(
            &mut field,
            &world,
            0,
            1,
            1,
            &mut seed,
            [HYDROGEN_DIFFUSION_K, OXYGEN_DIFFUSION_K],
            0.30,
        );

        assert!(field.amount(2, 1, GasKind::Hydrogen) <= 1e-6);
        assert!(field.amount(2, 1, GasKind::Oxygen) <= 1e-6);
    }

    #[test]
    fn scalar_d2q9_distribution_preserves_mass() {
        let distributed = distribute_scalar_d2q9_runtime(360.0);
        let sum: f32 = distributed.iter().sum();
        assert!((sum - 360.0).abs() < 1e-4);
        assert!((distributed[0] - 160.0).abs() < 1e-4);
    }

    #[test]
    fn antisymmetric_pair_flux_respects_limiter() {
        let mut field = GasField::default();
        let mut world = WorldGrid::default();
        field.clear_rect(UVec2::new(1, 1), UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2));

        field.set_amount(3, 3, GasKind::Hydrogen, 100.0);
        field.set_amount(4, 3, GasKind::Hydrogen, 10.0);
        assert!(world.set_solid(2, 3));
        assert!(world.set_solid(3, 2));
        assert!(world.set_solid(3, 4));

        let mut seed = 7;
        step_cpu_gas_block_sync(
            &mut field,
            &world,
            0,
            0,
            0,
            &mut seed,
            [0.20, 0.14],
            0.30,
        );

        let center = field.amount(3, 3, GasKind::Hydrogen);
        let right = field.amount(4, 3, GasKind::Hydrogen);
        assert!((center - 82.0).abs() < 1e-5);
        assert!((right - 28.0).abs() < 1e-5);
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
        field.clear_rect(UVec2::new(1, 1), UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2));
        field.set_amount(20, 20, GasKind::Hydrogen, 123.25);
        field.set_amount(20, 20, GasKind::Oxygen, 11.75);
        field.recompute_total_density_buffer(&world);
        field.sync_lbm_from_total_density(&world);
        let total_species = field.total_amount(20, 20);
        assert!((field.total_density(20, 20) - total_species).abs() < 1e-5);
    }
}


