use super::discrete_step::{step_discrete_in_place, DiscreteStepParams};
use super::gpu_solver::GpuSolverHostState;
use super::SolverTuning;
use crate::config::GasRegistry;
use crate::world::grid::{is_boundary, linear_index, WorldGrid, WORLD_HEIGHT, WORLD_WIDTH};
use bevy::prelude::*;
#[cfg(test)]
use std::sync::OnceLock;

pub const HYDROGEN_GPU_STORAGE_MAX_PARTICLES: u32 = 20_000;

const EPSILON: f32 = 1e-6;
#[cfg(test)]
const BUOYANCY_MIN_ENV_MASS: f32 = 1e-6;

pub type GasScalar = f32;
pub type GasCell = Vec<u32>;

#[derive(Clone, Debug)]
pub struct GasFieldSnapshot {
    pub gas_count: usize,
    pub species: Vec<u32>,
    pub total_density: Vec<f32>,
    pub velocity: Vec<[f32; 2]>,
}

#[derive(Clone, Copy)]
#[cfg(test)]
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
        let read = vec![vec![0u32; gas_count]; cells];
        Self {
            write: read.clone(),
            read,
            total_density: vec![0.0; cells],
            velocity: vec![Vec2::ZERO; cells],
            gas_count,
            molecular_masses: molecular_masses.to_vec(),
        }
    }

    pub fn gas_count(&self) -> usize {
        self.gas_count
    }

    pub fn snapshot_state(&self) -> GasFieldSnapshot {
        let cells = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
        let mut species = Vec::with_capacity(cells * self.gas_count);
        for idx in 0..cells {
            for gas_index in 0..self.gas_count {
                species.push(self.read[idx][gas_index]);
            }
        }

        let velocity = self.velocity.iter().map(|v| [v.x, v.y]).collect::<Vec<_>>();

        GasFieldSnapshot {
            gas_count: self.gas_count,
            species,
            total_density: self.total_density.clone(),
            velocity,
        }
    }

    pub fn restore_state(&mut self, snapshot: &GasFieldSnapshot) -> Result<(), String> {
        let cells = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
        if snapshot.gas_count != self.gas_count {
            return Err(format!(
                "Gas snapshot gas_count mismatch: got {}, expected {}",
                snapshot.gas_count, self.gas_count
            ));
        }

        if snapshot.species.len() != cells * self.gas_count {
            return Err(format!(
                "Gas snapshot species length mismatch: got {}, expected {}",
                snapshot.species.len(),
                cells * self.gas_count
            ));
        }
        if snapshot.total_density.len() != cells {
            return Err(format!(
                "Gas snapshot total_density length mismatch: got {}, expected {}",
                snapshot.total_density.len(),
                cells
            ));
        }
        if snapshot.velocity.len() != cells {
            return Err(format!(
                "Gas snapshot velocity length mismatch: got {}, expected {}",
                snapshot.velocity.len(),
                cells
            ));
        }

        for idx in 0..cells {
            for gas_index in 0..self.gas_count {
                let value = snapshot.species[idx * self.gas_count + gas_index];
                self.read[idx][gas_index] = value;
                self.write[idx][gas_index] = value;
            }

            let rho = snapshot.total_density[idx];
            if !rho.is_finite() {
                return Err(format!(
                    "Gas snapshot contains non-finite total_density at cell {}",
                    idx
                ));
            }
            self.total_density[idx] = rho.max(0.0);

            let vx = snapshot.velocity[idx][0];
            let vy = snapshot.velocity[idx][1];
            if !vx.is_finite() || !vy.is_finite() {
                return Err(format!(
                    "Gas snapshot contains non-finite velocity at cell {}",
                    idx
                ));
            }
            self.velocity[idx] = Vec2::new(vx, vy);
        }

        Ok(())
    }

    pub fn molecular_mass(&self, gas_index: usize) -> f32 {
        self.molecular_masses.get(gas_index).copied().unwrap_or(1.0)
    }

    pub fn amount_particles(&self, x: u32, y: u32, gas_index: usize) -> u32 {
        self.read[linear_index(x, y)][gas_index]
    }

    pub fn amount(&self, x: u32, y: u32, gas_index: usize) -> GasScalar {
        self.amount_particles(x, y, gas_index) as f32
    }

    pub fn amount_rounded(&self, x: u32, y: u32, gas_index: usize) -> u32 {
        self.amount_particles(x, y, gas_index)
    }

    pub fn total_amount_particles(&self, x: u32, y: u32) -> u64 {
        self.read[linear_index(x, y)]
            .iter()
            .map(|&v| u64::from(v))
            .sum()
    }

    pub fn total_amount(&self, x: u32, y: u32) -> GasScalar {
        self.total_amount_particles(x, y) as f32
    }

    pub fn total_amount_rounded(&self, x: u32, y: u32) -> u32 {
        self.total_amount_particles(x, y).min(u64::from(u32::MAX)) as u32
    }

    pub fn total_density(&self, x: u32, y: u32) -> f32 {
        self.total_density[linear_index(x, y)]
    }

    pub fn velocity(&self, x: u32, y: u32) -> Vec2 {
        self.velocity[linear_index(x, y)]
    }

    pub fn set_amount(&mut self, x: u32, y: u32, gas_index: usize, amount: GasScalar) {
        let index = linear_index(x, y);
        let clamped = amount.max(0.0).round().clamp(0.0, u32::MAX as f32) as u32;
        self.read[index][gas_index] = clamped;
        self.write[index][gas_index] = clamped;
    }

    pub fn clear_cell(&mut self, x: u32, y: u32) {
        let index = linear_index(x, y);
        for v in &mut self.read[index] {
            *v = 0;
        }
        for v in &mut self.write[index] {
            *v = 0;
        }
        self.total_density[index] = 0.0;
        self.velocity[index] = Vec2::ZERO;
    }

    pub fn clear_rect(&mut self, min: UVec2, max: UVec2) {
        for y in min.y..=max.y {
            for x in min.x..=max.x {
                self.clear_cell(x, y);
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

                let index = linear_index(x, y);
                if replace {
                    self.read[index][gas_index] = amount;
                    self.write[index][gas_index] = amount;
                } else {
                    self.read[index][gas_index] =
                        self.read[index][gas_index].saturating_add(amount);
                    self.write[index][gas_index] = self.read[index][gas_index];
                }
            }
        }
        self.recompute_total_density_buffer(world);
    }

    pub fn add_particles_no_impulse(
        &mut self,
        x: u32,
        y: u32,
        gas_index: usize,
        amount: u32,
        world: &WorldGrid,
    ) -> u32 {
        if amount == 0
            || gas_index >= self.gas_count
            || is_boundary(x, y)
            || world.is_solid(x, y)
        {
            return 0;
        }
        let idx = linear_index(x, y);
        let before = self.read[idx][gas_index];
        let after = before.saturating_add(amount);
        self.read[idx][gas_index] = after;
        self.write[idx][gas_index] = after;
        after.saturating_sub(before)
    }

    pub fn remove_particles_proportional(
        &mut self,
        x: u32,
        y: u32,
        amount: u32,
        world: &WorldGrid,
    ) -> u32 {
        if amount == 0 || is_boundary(x, y) || world.is_solid(x, y) {
            return 0;
        }

        let idx = linear_index(x, y);
        let species = &mut self.read[idx];
        let total: u64 = species.iter().map(|&v| u64::from(v)).sum();
        if total == 0 {
            return 0;
        }

        let remove = u64::from(amount).min(total);
        if remove == total {
            for v in species.iter_mut() {
                *v = 0;
            }
            self.write[idx].fill(0);
            return remove as u32;
        }

        let mut base_remove = vec![0u32; self.gas_count];
        let mut remainders = vec![0u64; self.gas_count];
        let mut removed_base = 0u64;

        for gas_index in 0..self.gas_count {
            let numerator = u128::from(species[gas_index]) * u128::from(remove);
            let base = (numerator / u128::from(total)) as u64;
            let remainder = (numerator % u128::from(total)) as u64;
            base_remove[gas_index] = base.min(u64::from(species[gas_index])) as u32;
            remainders[gas_index] = remainder;
            removed_base = removed_base.saturating_add(u64::from(base_remove[gas_index]));
        }

        let mut remaining = remove.saturating_sub(removed_base);
        while remaining > 0 {
            let mut best_index = None;
            let mut best_remainder = 0u64;
            for gas_index in 0..self.gas_count {
                if base_remove[gas_index] >= species[gas_index] {
                    continue;
                }
                let rem = remainders[gas_index];
                if best_index.is_none() || rem > best_remainder {
                    best_index = Some(gas_index);
                    best_remainder = rem;
                }
            }

            let Some(best) = best_index else {
                break;
            };
            base_remove[best] = base_remove[best].saturating_add(1);
            remainders[best] = 0;
            remaining -= 1;
        }

        let mut removed_total = 0u64;
        for gas_index in 0..self.gas_count {
            let remove_i = base_remove[gas_index].min(species[gas_index]);
            species[gas_index] = species[gas_index].saturating_sub(remove_i);
            self.write[idx][gas_index] = species[gas_index];
            removed_total = removed_total.saturating_add(u64::from(remove_i));
        }

        removed_total.min(u64::from(u32::MAX)) as u32
    }

    pub fn apply_species_delta_with_lbm(
        &mut self,
        x: u32,
        y: u32,
        gas_index: usize,
        delta: GasScalar,
    ) -> GasScalar {
        if delta.abs() <= EPSILON {
            return 0.0;
        }

        let index = linear_index(x, y);
        let before = self.read[index][gas_index] as i64;
        let delta_i = delta.round() as i64;
        if delta_i == 0 {
            return 0.0;
        }

        let unclamped = before.saturating_add(delta_i);
        let clamped = unclamped.clamp(0, i64::from(u32::MAX));
        self.read[index][gas_index] = clamped as u32;
        self.write[index][gas_index] = clamped as u32;
        (clamped - before) as f32
    }

    pub fn recompute_total_density_buffer(&mut self, world: &WorldGrid) {
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                let index = linear_index(x, y);
                if is_boundary(x, y) || world.is_solid(x, y) {
                    self.total_density[index] = 0.0;
                    self.velocity[index] = Vec2::ZERO;
                    continue;
                }
                self.total_density[index] = self.read[index].iter().map(|&v| v as f32).sum();
            }
        }
    }

    pub fn step_discrete(
        &mut self,
        world: &WorldGrid,
        tuning: &SolverTuning,
        thermal_motion_scale: f32,
        simulation_step: u64,
    ) {
        let cell_count = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
        let mut flat_read = vec![0u32; cell_count * self.gas_count];
        let mut flat_write = vec![0u32; cell_count * self.gas_count];

        for idx in 0..cell_count {
            let base = idx * self.gas_count;
            for gas_index in 0..self.gas_count {
                flat_read[base + gas_index] = self.read[idx][gas_index];
                flat_write[base + gas_index] = self.write[idx][gas_index];
            }
        }

        let solid_query = |x: u32, y: u32| world.is_solid(x, y);
        step_discrete_in_place(DiscreteStepParams {
            width: WORLD_WIDTH,
            height: WORLD_HEIGHT,
            gas_count: self.gas_count,
            molecular_masses: &self.molecular_masses,
            read: &mut flat_read,
            write: &mut flat_write,
            total_density: &mut self.total_density,
            velocity: &mut self.velocity,
            solid_query: &solid_query,
            temperature_multiplier: |_x, _y| 1.0,
            tuning,
            thermal_motion_scale,
            simulation_step,
        });

        for idx in 0..cell_count {
            let base = idx * self.gas_count;
            for gas_index in 0..self.gas_count {
                self.read[idx][gas_index] = flat_read[base + gas_index];
                self.write[idx][gas_index] = flat_write[base + gas_index];
            }
        }
    }

    pub fn species_totals(&self, world: &WorldGrid) -> Vec<GasScalar> {
        let mut totals = vec![0f64; self.gas_count];
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                if is_boundary(x, y) || world.is_solid(x, y) {
                    continue;
                }
                let idx = linear_index(x, y);
                for (gas_index, total) in totals.iter_mut().enumerate() {
                    *total += f64::from(self.read[idx][gas_index]);
                }
            }
        }
        totals.into_iter().map(|v| v as f32).collect()
    }

    pub fn species_totals_u64(&self, world: &WorldGrid) -> Vec<u64> {
        let mut totals = vec![0u64; self.gas_count];
        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                if is_boundary(x, y) || world.is_solid(x, y) {
                    continue;
                }
                let idx = linear_index(x, y);
                for (gas_index, total) in totals.iter_mut().enumerate() {
                    *total += u64::from(self.read[idx][gas_index]);
                }
            }
        }
        totals
    }

    pub fn to_gpu_host_state(&self, world: &WorldGrid) -> GpuSolverHostState {
        let cells = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
        let mut species = Vec::with_capacity(cells * self.gas_count);
        let mut total_density = Vec::with_capacity(cells);
        let mut velocity = Vec::with_capacity(cells);
        let mut solid_mask = Vec::with_capacity(cells);

        for y in 0..WORLD_HEIGHT {
            for x in 0..WORLD_WIDTH {
                let idx = linear_index(x, y);
                for gas_index in 0..self.gas_count {
                    species.push(self.read[idx][gas_index]);
                }
                total_density.push(self.total_density[idx]);
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

        GpuSolverHostState {
            gas_count: self.gas_count as u32,
            molecular_masses: self.molecular_masses.clone(),
            species,
            total_density,
            velocity,
            solid_mask,
        }
    }

    pub fn apply_gpu_host_state(&mut self, state: &GpuSolverHostState) {
        let cells = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
        if state.species.len() != cells.checked_mul(self.gas_count).unwrap_or(usize::MAX)
            || state.total_density.len() != cells
            || state.velocity.len() != cells
        {
            return;
        }
        if state.gas_count as usize != self.gas_count {
            return;
        }

        for idx in 0..cells {
            let base = idx * self.gas_count;
            for gas_index in 0..self.gas_count {
                let v = state.species[base + gas_index];
                self.read[idx][gas_index] = v;
                self.write[idx][gas_index] = v;
            }
            self.total_density[idx] = state.total_density[idx].max(0.0);
            self.velocity[idx] = Vec2::new(state.velocity[idx][0], state.velocity[idx][1]);
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{GameConfig, GasDefinition};
    use crate::save::{list_saves, load_save, saves_root_default};
    use crate::simulation::{do_one_substep, BlockSyncState, GasSimulationConfig, SimulationStep};

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
        let loaded = load_save(&saves_root, &equals.id, &registry).map_err(|e| e.to_string())?;

        let mut world = WorldGrid::default();
        world
            .restore_from_cell_codes(&loaded.state.world_cell_codes)
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
            let _ = world.set_solid_with_material(x, 20, crate::world::grid::CellMaterial::Brick);
            let _ = world.set_solid_with_material(x, 80, crate::world::grid::CellMaterial::Brick);
        }
        for y in 20..=80 {
            let _ = world.set_solid_with_material(20, y, crate::world::grid::CellMaterial::Brick);
            let _ = world.set_solid_with_material(80, y, crate::world::grid::CellMaterial::Brick);
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
    fn buoyancy_context_does_not_see_through_vertical_wall() {
        let registry = registry_with_three();
        let mut world_blocked = WorldGrid::default();
        let world_open = WorldGrid::default();

        for y in 10..=90 {
            let _ = world_blocked.set_solid_with_material(
                50,
                y,
                crate::world::grid::CellMaterial::Brick,
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
            crate::world::grid::CellMaterial::Brick,
        );
        let _ = world_blocked.set_solid_with_material(
            center_x,
            center_y + 1,
            crate::world::grid::CellMaterial::Brick,
        );
        let _ = world_blocked.set_solid_with_material(
            center_x - 1,
            center_y,
            crate::world::grid::CellMaterial::Brick,
        );
        let _ = world_blocked.set_solid_with_material(
            center_x + 1,
            center_y,
            crate::world::grid::CellMaterial::Brick,
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
