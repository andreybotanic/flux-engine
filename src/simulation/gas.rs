use bevy::prelude::*;
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::world::grid::{is_boundary, linear_index, WorldGrid, WORLD_HEIGHT, WORLD_WIDTH};

pub const HYDROGEN_DIFFUSION_NUMERATOR: u32 = 1;
pub const HYDROGEN_DIFFUSION_DENOMINATOR: u32 = 5;
pub const OXYGEN_DIFFUSION_NUMERATOR: u32 = 1;
pub const OXYGEN_DIFFUSION_DENOMINATOR: u32 = 7;
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
const D2Q9_DISCRETE_WEIGHTS: [u32; 9] = [16, 4, 4, 4, 4, 1, 1, 1, 1];
const D2Q9_WEIGHT_SUM: u32 = 36;
const RANDOM_LCG_MULTIPLIER: u64 = 6364136223846793005;
const RANDOM_LCG_INCREMENT: u64 = 1442695040888963407;

const EPSILON_DENSITY: f32 = 1e-6;
const ADVECTION_MAX_FRACTION: f32 = 0.45;
const ADVECTION_SPEED_SCALE: f32 = 0.55;
const LBM_VELOCITY_CLAMP: f32 = 0.95;

static RUNTIME_RANDOM_STATE: AtomicU64 = AtomicU64::new(0);

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
type GasCell = [u32; GAS_KIND_COUNT];

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
        let mut read = vec![[0; GAS_KIND_COUNT]; cells];

        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                let index = linear_index(x, y);
                read[index][GasKind::Hydrogen.index()] = seeded_hydrogen_amount(x, y);
                read[index][GasKind::Oxygen.index()] = seeded_oxygen_amount(x, y);
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
                let total = field.read[index]
                    .iter()
                    .copied()
                    .fold(0u32, |acc, value| acc.saturating_add(value));
                field.total_density[index] = total as f32;
                if total == 0 {
                    continue;
                }

                let distribution = distribute_amount_d2q9_runtime(total);
                for (dir, amount) in distribution.into_iter().enumerate() {
                    let value = amount as f32;
                    field.lbm_read[index][dir] = value;
                    field.lbm_write[index][dir] = value;
                }
                field.velocity[index] = Vec2::ZERO;
            }
        }

        field
    }
}

impl GasField {
    pub fn amount(&self, x: u32, y: u32, kind: GasKind) -> u32 {
        self.read[linear_index(x, y)][kind.index()]
    }

    pub fn total_amount(&self, x: u32, y: u32) -> u32 {
        self.read[linear_index(x, y)]
            .iter()
            .copied()
            .fold(0u32, |acc, value| acc.saturating_add(value))
    }

    pub fn total_density(&self, x: u32, y: u32) -> f32 {
        self.total_density[linear_index(x, y)]
    }

    pub fn velocity(&self, x: u32, y: u32) -> Vec2 {
        self.velocity[linear_index(x, y)]
    }

    pub fn set_amount(&mut self, x: u32, y: u32, kind: GasKind, amount: u32) {
        let index = linear_index(x, y);
        self.read[index][kind.index()] = amount;
        self.write[index][kind.index()] = amount;
    }

    pub fn add_amount(&mut self, x: u32, y: u32, kind: GasKind, amount: u32) {
        let current = self.amount(x, y, kind);
        self.set_amount(x, y, kind, current.saturating_add(amount));
    }

    pub fn clear_amount(&mut self, x: u32, y: u32, kind: GasKind) {
        self.set_amount(x, y, kind, 0);
    }

    pub fn clear_cell(&mut self, x: u32, y: u32) {
        let index = linear_index(x, y);
        self.read[index] = [0; GAS_KIND_COUNT];
        self.write[index] = [0; GAS_KIND_COUNT];
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
                    if amount > 0 {
                        self.apply_species_delta_with_lbm(x, y, kind, -(i64::from(amount)));
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
                    let current = i64::from(self.amount(x, y, kind));
                    let target = i64::from(amount);
                    self.apply_species_delta_with_lbm(x, y, kind, target - current);
                } else {
                    self.apply_species_delta_with_lbm(x, y, kind, i64::from(amount));
                }
            }
        }
    }

    pub fn apply_species_delta_with_lbm(
        &mut self,
        x: u32,
        y: u32,
        kind: GasKind,
        delta: i64,
    ) -> i64 {
        if delta == 0 {
            return 0;
        }

        let index = linear_index(x, y);
        let kind_index = kind.index();
        let before = i64::from(self.read[index][kind_index]);
        let unclamped = before.saturating_add(delta);
        let clamped = unclamped.clamp(0, i64::from(u32::MAX));
        let applied = clamped - before;
        if applied == 0 {
            return 0;
        }

        let after = clamped as u32;
        self.read[index][kind_index] = after;
        self.write[index][kind_index] = after;
        self.apply_total_lbm_delta_at_index(index, applied);
        applied
    }

    fn apply_total_lbm_delta_at_index(&mut self, index: usize, delta: i64) {
        if delta == 0 {
            return;
        }

        if delta > 0 {
            let distribution = distribute_amount_d2q9_runtime(delta as u32);
            for (dir, amount) in distribution.into_iter().enumerate() {
                self.lbm_read[index][dir] += amount as f32;
            }
        } else {
            let current_total: f32 = self.lbm_read[index].iter().sum();
            if current_total <= EPSILON_DENSITY {
                self.lbm_read[index] = [0.0; 9];
            } else {
                let remove = (-delta) as f32;
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
                self.total_density[index] = (cell[0] as f32) + (cell[1] as f32);
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

    pub fn step_lbm(&mut self, world: &WorldGrid, tau: f32) {
        for entry in &mut self.lbm_write {
            *entry = [0.0; 9];
        }
        for entry in &mut self.write {
            *entry = [0; GAS_KIND_COUNT];
        }

        let omega = (1.0 / tau.max(0.55)).min(1.99);

        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                let index = linear_index(x, y);
                if is_boundary(x, y) || world.is_solid(x, y) {
                    self.velocity[index] = Vec2::ZERO;
                    self.write[index] = [0; GAS_KIND_COUNT];
                    continue;
                }

                let (rho, u) = macroscopic_from_distributions(&self.lbm_read[index]);
                let feq = equilibrium_distributions(rho, u);

                let mut post = [0.0; 9];
                for i in 0..9 {
                    post[i] = self.lbm_read[index][i] - omega * (self.lbm_read[index][i] - feq[i]);
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
                    if amount == 0 {
                        continue;
                    }

                    if post_sum <= EPSILON_DENSITY {
                        self.write[index][kind_index] = self.write[index][kind_index].saturating_add(amount);
                        continue;
                    }

                    let shares = distribute_integer_by_float_weights(amount, &post, post_sum);
                    for (dir, share) in shares.into_iter().enumerate() {
                        if share == 0 {
                            continue;
                        }
                        let target_index = species_targets[dir];
                        self.write[target_index][kind_index] =
                            self.write[target_index][kind_index].saturating_add(share);
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

    pub fn advect_species_with_velocity(&mut self, world: &WorldGrid) {
        self.write.copy_from_slice(&self.read);

        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                let index = linear_index(x, y);
                if is_boundary(x, y) || world.is_solid(x, y) {
                    self.write[index] = [0; GAS_KIND_COUNT];
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
                    if amount == 0 {
                        continue;
                    }

                    let total_move = ((amount as f32) * move_fraction).round() as u32;
                    if total_move == 0 {
                        continue;
                    }

                    let mut moved_sum = 0u32;
                    for (slot, (target, weight)) in targets[..target_count].iter().enumerate() {
                        let Some((tx, ty)) = *target else {
                            continue;
                        };
                        let neighbour_index = linear_index(tx, ty);

                        let share = if slot + 1 == target_count {
                            total_move.saturating_sub(moved_sum)
                        } else {
                            (((total_move as f32) * (*weight / total_weight)).round() as u32)
                                .min(total_move.saturating_sub(moved_sum))
                        };

                        if share == 0 {
                            continue;
                        }

                        self.write[index][kind_index] = self.write[index][kind_index].saturating_sub(share);
                        self.write[neighbour_index][kind_index] = self.write[neighbour_index][kind_index].saturating_add(share);
                        moved_sum = moved_sum.saturating_add(share);
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
) {
    gas.write.copy_from_slice(&gas.read);

    let target_x = (block_index % 3) as u32;
    let target_y = (block_index / 3) as u32;

    for y in 0..WORLD_HEIGHT {
        for x in 0..WORLD_WIDTH {
            let index = linear_index(x, y);
            if is_boundary(x, y) || world.is_solid(x, y) {
                gas.write[index] = [0; GAS_KIND_COUNT];
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
                let center = gas.read[index][kind_index];
                let center_transfer = movable_particles(center, kind, rng_state);

                let neighbour_transfer = movable_particles(gas.read[neighbour_index][kind_index], kind, rng_state);

                gas.write[index][kind_index] = gas.write[index][kind_index]
                    .saturating_sub(center_transfer)
                    .saturating_add(neighbour_transfer);
                gas.write[neighbour_index][kind_index] = gas.write[neighbour_index][kind_index]
                    .saturating_sub(neighbour_transfer)
                    .saturating_add(center_transfer);
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

fn ensure_runtime_random_seed() -> u64 {
    let current = RUNTIME_RANDOM_STATE.load(Ordering::Relaxed);
    if current != 0 {
        return current;
    }

    let seeded = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0x0dd5_eed0_beef_cafe)
        | 1;

    match RUNTIME_RANDOM_STATE.compare_exchange(0, seeded, Ordering::Relaxed, Ordering::Relaxed) {
        Ok(_) => seeded,
        Err(existing) => existing,
    }
}

fn next_runtime_random_bounded(bound_exclusive: u32) -> u32 {
    if bound_exclusive <= 1 {
        return 0;
    }

    let mut state = ensure_runtime_random_seed();
    loop {
        let next = state
            .wrapping_mul(RANDOM_LCG_MULTIPLIER)
            .wrapping_add(RANDOM_LCG_INCREMENT);
        match RUNTIME_RANDOM_STATE.compare_exchange_weak(
            state,
            next,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return ((next >> 32) as u32) % bound_exclusive,
            Err(actual) => state = actual,
        }
    }
}

fn pick_d2q9_direction(mut ticket: u32) -> usize {
    for (dir, weight) in D2Q9_DISCRETE_WEIGHTS.into_iter().enumerate() {
        if ticket < weight {
            return dir;
        }
        ticket -= weight;
    }
    0
}

fn distribute_amount_d2q9_with_draw(mut amount: u32, mut draw: impl FnMut(u32) -> u32) -> [u32; 9] {
    let mut result = [0u32; 9];
    let base = amount / D2Q9_WEIGHT_SUM;
    amount %= D2Q9_WEIGHT_SUM;

    for (dir, weight) in D2Q9_DISCRETE_WEIGHTS.into_iter().enumerate() {
        result[dir] = base.saturating_mul(weight);
    }

    for _ in 0..amount {
        let ticket = draw(D2Q9_WEIGHT_SUM);
        let dir = pick_d2q9_direction(ticket);
        result[dir] = result[dir].saturating_add(1);
    }

    result
}

fn distribute_amount_d2q9_runtime(amount: u32) -> [u32; 9] {
    distribute_amount_d2q9_with_draw(amount, next_runtime_random_bounded)
}

fn distribute_integer_by_float_weights(total: u32, weights: &[f32; 9], sum_weights: f32) -> [u32; 9] {
    if total == 0 || sum_weights <= EPSILON_DENSITY {
        return [0; 9];
    }

    let mut result = [0u32; 9];
    let mut remainders = [(0usize, 0.0f32); 9];
    let mut assigned = 0u32;

    for i in 0..9 {
        let raw = (total as f32) * (weights[i] / sum_weights);
        let base = raw.floor() as u32;
        result[i] = base;
        assigned = assigned.saturating_add(base);
        remainders[i] = (i, raw - base as f32);
    }

    let mut remaining = total.saturating_sub(assigned) as usize;
    remainders.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let mut idx = 0usize;
    while remaining > 0 {
        let direction = remainders[idx % 9].0;
        result[direction] = result[direction].saturating_add(1);
        remaining -= 1;
        idx += 1;
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

            let has_movable = GasKind::ALL.into_iter().any(|kind| {
                movable_particles(gas.amount(x, y, kind), kind, &mut rng_state) > 0
            });
            if !has_movable {
                continue;
            }

            let pick = (next_random_u32(&mut rng_state) as usize) % neighbours.len();
            let (nx, ny) = neighbours[pick];
            moves.push((x, y, nx, ny));
        }
    }

    (block_index, moves)
}

fn movable_particles(amount: u32, kind: GasKind, rng_state: &mut u64) -> u32 {
    if amount == 0 {
        return 0;
    }

    let (numerator_coeff, denominator) = diffusion_coeff(kind);
    let numerator = amount.saturating_mul(numerator_coeff);

    if numerator < denominator {
        let draw = next_random_u32(rng_state) % denominator;
        return if draw < numerator { 1 } else { 0 };
    }

    let rounded = numerator.saturating_add(denominator / 2) / denominator;
    rounded.min(amount)
}

fn diffusion_coeff(kind: GasKind) -> (u32, u32) {
    match kind {
        GasKind::Hydrogen => (HYDROGEN_DIFFUSION_NUMERATOR, HYDROGEN_DIFFUSION_DENOMINATOR),
        GasKind::Oxygen => (OXYGEN_DIFFUSION_NUMERATOR, OXYGEN_DIFFUSION_DENOMINATOR),
    }
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

    fn total_species(field: &GasField, kind: GasKind) -> u64 {
        field
            .read
            .iter()
            .map(|cell| u64::from(cell[kind.index()]))
            .sum()
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
        field.set_amount(10, 10, GasKind::Hydrogen, 17);
        field.set_amount(10, 10, GasKind::Oxygen, 23);

        field.recompute_total_density_buffer(&world);

        assert_eq!(field.total_density(10, 10), 40.0);
    }

    #[test]
    fn diffusion_moves_each_species_independently_but_same_pair_step() {
        let mut field = GasField::default();
        let world = WorldGrid::default();

        field.clear_rect(UVec2::new(1, 1), UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2));
        field.set_amount(4, 4, GasKind::Hydrogen, 100);
        field.set_amount(4, 4, GasKind::Oxygen, 100);

        let h_before = total_species(&field, GasKind::Hydrogen);
        let o_before = total_species(&field, GasKind::Oxygen);

        let mut seed = 1234;
        step_cpu_gas_block_sync(&mut field, &world, 8, 1, 1, &mut seed);

        assert_eq!(h_before, total_species(&field, GasKind::Hydrogen));
        assert_eq!(o_before, total_species(&field, GasKind::Oxygen));
    }

    #[test]
    fn solids_block_all_species_diffusion() {
        let mut field = GasField::default();
        let mut world = WorldGrid::default();

        field.clear_rect(UVec2::new(1, 1), UVec2::new(WORLD_WIDTH - 2, WORLD_HEIGHT - 2));
        field.set_amount(1, 1, GasKind::Hydrogen, 50);
        field.set_amount(1, 1, GasKind::Oxygen, 50);
        assert!(world.set_solid(2, 1));

        let mut seed = 5;
        step_cpu_gas_block_sync(&mut field, &world, 0, 1, 1, &mut seed);

        assert_eq!(field.amount(2, 1, GasKind::Hydrogen), 0);
        assert_eq!(field.amount(2, 1, GasKind::Oxygen), 0);
    }

    #[test]
    fn d2q9_distribution_for_multiple_of_36_matches_exact_16_4_1_pattern() {
        let distributed = distribute_amount_d2q9_with_draw(360, |_| 0);

        assert_eq!(distributed[0], 160);
        assert_eq!(distributed[1], 40);
        assert_eq!(distributed[2], 40);
        assert_eq!(distributed[3], 40);
        assert_eq!(distributed[4], 40);
        assert_eq!(distributed[5], 10);
        assert_eq!(distributed[6], 10);
        assert_eq!(distributed[7], 10);
        assert_eq!(distributed[8], 10);
        assert_eq!(distributed.iter().copied().sum::<u32>(), 360);
    }

    #[test]
    fn d2q9_distribution_for_non_multiple_of_36_preserves_sum_and_base() {
        let mut rng_state = 123u64;
        let amount = 101u32;
        let distributed = distribute_amount_d2q9_with_draw(amount, |bound| next_random_u32(&mut rng_state) % bound);

        assert_eq!(distributed.iter().copied().sum::<u32>(), amount);
        let base = amount / D2Q9_WEIGHT_SUM;
        for (dir, weight) in D2Q9_DISCRETE_WEIGHTS.into_iter().enumerate() {
            assert!(distributed[dir] >= base * weight);
        }
    }

    #[test]
    fn positive_delta_injects_zero_velocity_component_for_multiple_of_36() {
        let mut field = GasField::default();
        let x = 12;
        let y = 12;
        field.clear_cell(x, y);
        let index = linear_index(x, y);

        let applied = field.apply_species_delta_with_lbm(x, y, GasKind::Hydrogen, 360);
        assert_eq!(applied, 360);

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
        field.set_amount(x, y, GasKind::Hydrogen, 720);
        field.total_density[index] = 720.0;
        field.velocity[index] = Vec2::new(0.22, -0.11);
        let eq = equilibrium_distributions(720.0, field.velocity[index]);
        field.lbm_read[index] = eq;
        field.lbm_write[index] = eq;

        let before = macroscopic_from_distributions(&field.lbm_read[index]).1;
        let applied = field.apply_species_delta_with_lbm(x, y, GasKind::Hydrogen, -180);
        assert_eq!(applied, -180);
        let after = macroscopic_from_distributions(&field.lbm_read[index]).1;

        assert!(after.x * before.x >= 0.0);
        assert!(after.y * before.y >= 0.0);
        assert!((after.x - before.x).abs() < 1e-4);
        assert!((after.y - before.y).abs() < 1e-4);
        assert!(field.lbm_read[index].iter().all(|v| *v >= -1e-6));
    }
}


