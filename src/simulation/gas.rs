use bevy::prelude::*;

use crate::world::grid::{is_boundary, linear_index, WorldGrid, WORLD_HEIGHT, WORLD_WIDTH};

pub const HYDROGEN_DIFFUSION_NUMERATOR: u32 = 1;
pub const HYDROGEN_DIFFUSION_DENOMINATOR: u32 = 5;
pub const HYDROGEN_MAX_VISUAL_PARTICLES: u32 = 220;
pub const INITIAL_HYDROGEN_CENTER_PARTICLES: u32 = 10_000;
pub const HYDROGEN_GPU_STORAGE_MAX_PARTICLES: u32 = INITIAL_HYDROGEN_CENTER_PARTICLES;

#[derive(Resource, Clone)]
pub struct GasField {
    pub read: Vec<u32>,
    write: Vec<u32>,
}

impl Default for GasField {
    fn default() -> Self {
        let mut read = vec![0; (WORLD_WIDTH * WORLD_HEIGHT) as usize];
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                read[linear_index(x, y)] = seeded_gas_amount(x, y);
            }
        }
        Self {
            write: read.clone(),
            read,
        }
    }
}

impl GasField {
    pub fn amount(&self, x: u32, y: u32) -> u32 {
        self.read[linear_index(x, y)]
    }

    pub fn set_amount(&mut self, x: u32, y: u32, amount: u32) {
        let index = linear_index(x, y);
        self.read[index] = amount;
        self.write[index] = amount;
    }

    pub fn add_amount(&mut self, x: u32, y: u32, amount: u32) {
        let next = self.amount(x, y).saturating_add(amount);
        self.set_amount(x, y, next);
    }

    pub fn clear_amount(&mut self, x: u32, y: u32) {
        self.set_amount(x, y, 0);
    }

    pub fn clear_rect(&mut self, min: UVec2, max: UVec2) {
        for y in min.y..=max.y {
            for x in min.x..=max.x {
                self.clear_amount(x, y);
            }
        }
    }

    pub fn apply_rect(
        &mut self,
        min: UVec2,
        max: UVec2,
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
                    self.set_amount(x, y, amount);
                } else {
                    self.add_amount(x, y, amount);
                }
            }
        }
    }
}

pub fn seeded_gas_amount(x: u32, y: u32) -> u32 {
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
            if is_boundary(x, y) {
                gas.write[index] = 0;
                continue;
            }
            if world.is_solid(x, y) {
                gas.write[index] = 0;
                continue;
            }

            if !is_active_block_cell(x, y, target_x, target_y, offset_x, offset_y) {
                continue;
            }

            let center = gas.read[index];
            let center_transfer = movable_particles(center, rng_state);
            if center_transfer == 0 {
                continue;
            }

            let neighbours = open_cardinal_neighbours(world, x, y);
            if neighbours.is_empty() {
                continue;
            }

            let pick = (next_random_u32(rng_state) as usize) % neighbours.len();
            let (nx, ny) = neighbours[pick];
            let neighbour_index = linear_index(nx, ny);

            // Diffusion is symmetric for the selected pair: each cell keeps its retained
            // fraction and sends the remainder to the other cell.
            let neighbour_transfer = movable_particles(gas.read[neighbour_index], rng_state);

            gas.write[index] = gas.write[index]
                .saturating_sub(center_transfer)
                .saturating_add(neighbour_transfer);
            gas.write[neighbour_index] = gas.write[neighbour_index]
                .saturating_sub(neighbour_transfer)
                .saturating_add(center_transfer);
        }
    }

    std::mem::swap(&mut gas.read, &mut gas.write);
}

pub fn next_random_u32(state: &mut u64) -> u32 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*state >> 32) as u32
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

/// Preview what the next substep would do without modifying the gas field.
/// `rng_state` is passed by value (copied).
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
            if is_boundary(x, y) {
                continue;
            }
            if world.is_solid(x, y) {
                continue;
            }
            if !is_active_block_cell(x, y, target_x, target_y, offset_x, offset_y) {
                continue;
            }
            let center = gas.amount(x, y);
            let transfer = movable_particles(center, &mut rng_state);
            if transfer == 0 {
                continue;
            }
            let neighbours = open_cardinal_neighbours(world, x, y);
            if neighbours.is_empty() {
                continue;
            }
            let pick = (next_random_u32(&mut rng_state) as usize) % neighbours.len();
            let (nx, ny) = neighbours[pick];
            moves.push((x, y, nx, ny));
        }
    }

    (block_index, moves)
}

fn movable_particles(amount: u32, rng_state: &mut u64) -> u32 {
    if amount == 0 {
        return 0;
    }

    let numerator = amount.saturating_mul(HYDROGEN_DIFFUSION_NUMERATOR);

    // For low concentrations where expected transfer is below one particle,
    // transfer one particle stochastically with probability p = coeff * amount.
    if numerator < HYDROGEN_DIFFUSION_DENOMINATOR {
        let draw = next_random_u32(rng_state) % HYDROGEN_DIFFUSION_DENOMINATOR;
        return if draw < numerator { 1 } else { 0 };
    }

    let rounded = numerator
        .saturating_add(HYDROGEN_DIFFUSION_DENOMINATOR / 2)
        / HYDROGEN_DIFFUSION_DENOMINATOR;
    rounded.min(amount)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::grid::linear_index;

    fn total_particles(field: &GasField) -> u64 {
        field.read.iter().map(|&v| u64::from(v)).sum()
    }

    #[test]
    fn seeds_center_with_expected_particles() {
        let center_x = WORLD_WIDTH / 2;
        let center_y = WORLD_HEIGHT / 2;

        assert_eq!(seeded_gas_amount(center_x, center_y), INITIAL_HYDROGEN_CENTER_PARTICLES);
        assert_eq!(seeded_gas_amount(center_x + 1, center_y), 0);
    }

    #[test]
    fn one_step_spreads_particles_to_cardinal_neighbours() {
        let mut field = GasField::default();
        let world = WorldGrid::default();
        let center_x = WORLD_WIDTH / 2;
        let center_y = WORLD_HEIGHT / 2;
        let center_before = field.amount(center_x, center_y);
        let mut rng_state = 123;

        step_cpu_gas_block_sync(&mut field, &world, 0, 0, 0, &mut rng_state);

        assert!(field.amount(center_x, center_y) < center_before);

        let neighbour_sum = field.amount(center_x + 1, center_y)
            + field.amount(center_x - 1, center_y)
            + field.amount(center_x, center_y + 1)
            + field.amount(center_x, center_y - 1);
        assert!(neighbour_sum > 0);
    }

    #[test]
    fn step_preserves_total_particle_count() {
        let mut field = GasField::default();
        let world = WorldGrid::default();
        let before = total_particles(&field);
        let mut rng_state = 777;

        for step in 0..32 {
            let block_index = (next_random_u32(&mut rng_state) % 9) as u8;
            let offset_x = step % 3;
            let offset_y = (step / 3) % 3;
            step_cpu_gas_block_sync(&mut field, &world, block_index, offset_x, offset_y, &mut rng_state);
        }

        let after = total_particles(&field);
        assert_eq!(before, after);

        let center_x = WORLD_WIDTH / 2;
        let center_y = WORLD_HEIGHT / 2;
        assert_eq!(
            field.read[linear_index(center_x, center_y)],
            field.amount(center_x, center_y)
        );
    }

    #[test]
    fn cyclic_offsets_keep_activation_counts_uniform() {
        let mut counts = vec![0u32; (WORLD_WIDTH * WORLD_HEIGHT) as usize];

        for phase in 0..9u32 {
            let offset_x = phase % 3;
            let offset_y = (phase / 3) % 3;
            for block_index in 0..9u32 {
                let target_x = block_index % 3;
                let target_y = block_index / 3;
                for y in 1..WORLD_HEIGHT - 1 {
                    for x in 1..WORLD_WIDTH - 1 {
                        if is_active_block_cell(x, y, target_x, target_y, offset_x, offset_y) {
                            counts[linear_index(x, y)] += 1;
                        }
                    }
                }
            }
        }

        for y in 1..WORLD_HEIGHT - 1 {
            for x in 1..WORLD_WIDTH - 1 {
                assert_eq!(counts[linear_index(x, y)], 9);
            }
        }
    }

    #[test]
    fn single_particle_can_move_or_stay_stochastically() {
        let mut field = GasField {
            read: vec![0; (WORLD_WIDTH * WORLD_HEIGHT) as usize],
            write: vec![0; (WORLD_WIDTH * WORLD_HEIGHT) as usize],
        };

        let x = 4;
        let y = 4;
        field.read[linear_index(x, y)] = 1;
        field.write.copy_from_slice(&field.read);

        // Find a seed where the first draw % 5 == 0, so 1 particle moves with p=0.2.
        let mut move_seed = 0u64;
        while {
            let mut probe = move_seed;
            next_random_u32(&mut probe) % HYDROGEN_DIFFUSION_DENOMINATOR != 0
        } {
            move_seed = move_seed.wrapping_add(1);
        }

        // Find a seed where first draw % 5 != 0, so the particle stays.
        let mut stay_seed = 0u64;
        while {
            let mut probe = stay_seed;
            next_random_u32(&mut probe) % HYDROGEN_DIFFUSION_DENOMINATOR == 0
        } {
            stay_seed = stay_seed.wrapping_add(1);
        }

        let mut moved_case = field.clone();
        let world = WorldGrid::default();
        step_cpu_gas_block_sync(&mut moved_case, &world, 0, 1, 1, &mut move_seed);
        assert_eq!(moved_case.amount(x, y), 0);
        let moved_neighbour_sum = moved_case.amount(x + 1, y)
            + moved_case.amount(x - 1, y)
            + moved_case.amount(x, y + 1)
            + moved_case.amount(x, y - 1);
        assert_eq!(moved_neighbour_sum, 1);

        let mut stayed_case = field;
        step_cpu_gas_block_sync(&mut stayed_case, &world, 0, 1, 1, &mut stay_seed);
        assert_eq!(stayed_case.amount(x, y), 1);
        let stayed_neighbour_sum = stayed_case.amount(x + 1, y)
            + stayed_case.amount(x - 1, y)
            + stayed_case.amount(x, y + 1)
            + stayed_case.amount(x, y - 1);
        assert_eq!(stayed_neighbour_sum, 0);
    }

    #[test]
    fn pair_exchange_is_bidirectional_with_inverted_coefficient() {
        let mut field = GasField {
            read: vec![0; (WORLD_WIDTH * WORLD_HEIGHT) as usize],
            write: vec![0; (WORLD_WIDTH * WORLD_HEIGHT) as usize],
        };

        // Active cell (1,1) exchanges with right neighbor (2,1).
        let x1 = 1;
        let y1 = 1;
        let x2 = 2;
        let y2 = 1;
        field.read[linear_index(x1, y1)] = 15;
        field.read[linear_index(x2, y2)] = 50;
        field.write.copy_from_slice(&field.read);

        // For (1,1), open neighbors are [right, down]; pick right (index 0) by choosing
        // a seed whose first RNG output is even.
        let mut rng_state = 0u64;
        while {
            let mut probe = rng_state;
            next_random_u32(&mut probe) % 2 != 0
        } {
            rng_state = rng_state.wrapping_add(1);
        }

        // offset(1,1) + block_index 0 activates cells where x%3==1 and y%3==1, including (1,1).
        let world = WorldGrid::default();
        step_cpu_gas_block_sync(&mut field, &world, 0, 1, 1, &mut rng_state);

        // Transfer 20% and retain 80% each way:
        // cell1: 15 -> sends 3, keeps 12
        // cell2: 50 -> sends 10, keeps 40
        // result: cell1=12+10=22, cell2=40+3=43
        assert_eq!(field.amount(x1, y1), 22);
        assert_eq!(field.amount(x2, y2), 43);
    }

    #[test]
    fn pair_5_and_8_rounds_to_6_and_7() {
        let mut field = GasField {
            read: vec![0; (WORLD_WIDTH * WORLD_HEIGHT) as usize],
            write: vec![0; (WORLD_WIDTH * WORLD_HEIGHT) as usize],
        };

        let x1 = 1;
        let y1 = 1;
        let x2 = 2;
        let y2 = 1;
        field.read[linear_index(x1, y1)] = 5;
        field.read[linear_index(x2, y2)] = 8;
        field.write.copy_from_slice(&field.read);

        let mut rng_state = 0u64;
        while {
            let mut probe = rng_state;
            next_random_u32(&mut probe) % 2 != 0
        } {
            rng_state = rng_state.wrapping_add(1);
        }

        let world = WorldGrid::default();
        step_cpu_gas_block_sync(&mut field, &world, 0, 1, 1, &mut rng_state);

        assert_eq!(field.amount(x1, y1), 6);
        assert_eq!(field.amount(x2, y2), 7);
    }

    #[test]
    fn diffusion_does_not_cross_solid_cells() {
        let mut field = GasField {
            read: vec![0; (WORLD_WIDTH * WORLD_HEIGHT) as usize],
            write: vec![0; (WORLD_WIDTH * WORLD_HEIGHT) as usize],
        };
        let mut world = WorldGrid::default();

        // Source at (1,1) and destination candidate at (2,1).
        let source = (1, 1);
        let blocked = (2, 1);
        field.set_amount(source.0, source.1, 10);
        assert!(world.set_solid(blocked.0, blocked.1));

        let mut rng_state = 0u64;

        step_cpu_gas_block_sync(&mut field, &world, 0, 1, 1, &mut rng_state);

        assert_eq!(field.amount(blocked.0, blocked.1), 0);
    }
}
