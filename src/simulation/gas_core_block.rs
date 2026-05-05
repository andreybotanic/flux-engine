impl GasField {
/// Runs `from_registry` logic.
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

/// Runs `gas_count` logic.
    pub fn gas_count(&self) -> usize {
        self.gas_count
    }

/// Runs `snapshot_state` logic.
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

/// Runs `restore_state` logic.
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

/// Runs `molecular_mass` logic.
    pub fn molecular_mass(&self, gas_index: usize) -> f32 {
        self.molecular_masses.get(gas_index).copied().unwrap_or(1.0)
    }

/// Runs `amount_particles` logic.
    pub fn amount_particles(&self, x: u32, y: u32, gas_index: usize) -> u32 {
        self.read[linear_index(x, y)][gas_index]
    }

/// Runs `amount` logic.
    pub fn amount(&self, x: u32, y: u32, gas_index: usize) -> GasScalar {
        self.amount_particles(x, y, gas_index) as f32
    }

/// Runs `amount_rounded` logic.
    pub fn amount_rounded(&self, x: u32, y: u32, gas_index: usize) -> u32 {
        self.amount_particles(x, y, gas_index)
    }

/// Runs `total_amount_particles` logic.
    pub fn total_amount_particles(&self, x: u32, y: u32) -> u64 {
        self.read[linear_index(x, y)]
            .iter()
            .map(|&v| u64::from(v))
            .sum()
    }

/// Runs `total_amount` logic.
    pub fn total_amount(&self, x: u32, y: u32) -> GasScalar {
        self.total_amount_particles(x, y) as f32
    }

/// Runs `total_amount_rounded` logic.
    pub fn total_amount_rounded(&self, x: u32, y: u32) -> u32 {
        self.total_amount_particles(x, y).min(u64::from(u32::MAX)) as u32
    }

/// Runs `total_density` logic.
    pub fn total_density(&self, x: u32, y: u32) -> f32 {
        self.total_density[linear_index(x, y)]
    }

/// Runs `velocity` logic.
    pub fn velocity(&self, x: u32, y: u32) -> Vec2 {
        self.velocity[linear_index(x, y)]
    }

/// Runs `set_amount` logic.
    pub fn set_amount(&mut self, x: u32, y: u32, gas_index: usize, amount: GasScalar) {
        let index = linear_index(x, y);
        let clamped = amount.max(0.0).round().clamp(0.0, u32::MAX as f32) as u32;
        self.read[index][gas_index] = clamped;
        self.write[index][gas_index] = clamped;
    }

/// Runs `clear_cell` logic.
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

/// Runs `clear_rect` logic.
    pub fn clear_rect(&mut self, min: UVec2, max: UVec2) {
        for y in min.y..=max.y {
            for x in min.x..=max.x {
                self.clear_cell(x, y);
            }
        }
    }

/// Runs `apply_rect` logic.
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

/// Runs `add_particles_no_impulse` logic.
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

/// Runs `remove_particles_proportional` logic.
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

/// Runs `apply_species_delta_with_lbm` logic.
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

/// Runs `recompute_total_density_buffer` logic.
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

/// Runs `step_discrete` logic.
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

/// Runs `species_totals` logic.
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

/// Runs `species_totals_u64` logic.
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

/// Runs `to_gpu_host_state` logic.
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

/// Runs `apply_gpu_host_state` logic.
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

