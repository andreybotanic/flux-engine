impl GpuGasSolver {
    pub fn step(&mut self, params: ParamsPod) -> Result<GpuStepTimings, String> {
        let total_started = Instant::now();
        if params.gas_count == 0 {
            return Err("GPU step received gas_count=0".to_string());
        }
        if params.gas_count > self.gas_capacity {
            return Err(format!(
                "GPU step gas_count {} exceeds uploaded capacity {}",
                params.gas_count, self.gas_capacity
            ));
        }
        self.gas_count = params.gas_count;

        self.queue
            .write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&params));

        let cells = self.width.saturating_mul(self.height);
        let workgroups = (cells + 63) / 64;

        let compute_started = Instant::now();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("gas-solver-step-encoder"),
            });

        let mut timestamp_writes = None;
        if self.timestamp_enabled && self.timestamp_inside_passes {
            if let Some(query_set) = &self.timestamp_query_set {
                timestamp_writes = Some(wgpu::ComputePassTimestampWrites {
                    query_set,
                    beginning_of_pass_write_index: Some(0),
                    end_of_pass_write_index: Some(1),
                });
            }
        } else if self.timestamp_enabled && self.timestamp_inside_encoders {
            if let Some(query_set) = &self.timestamp_query_set {
                encoder.write_timestamp(query_set, 0);
            }
        }

        dispatch_pipeline(
            &mut encoder,
            &self.pipelines.compute_shares,
            &self.bind_groups[self.active_index],
            workgroups.max(1),
            None,
        );

        dispatch_pipeline(
            &mut encoder,
            &self.pipelines.step_discrete_exact,
            &self.bind_groups[self.active_index],
            workgroups.max(1),
            timestamp_writes.take(),
        );

        self.active_index = 1 - self.active_index;

        if self.timestamp_enabled && !self.timestamp_inside_passes && self.timestamp_inside_encoders
        {
            if let Some(query_set) = &self.timestamp_query_set {
                encoder.write_timestamp(query_set, 1);
            }
        }

        if let (Some(query_set), Some(resolve), Some(readback)) = (
            &self.timestamp_query_set,
            &self.timestamp_resolve_buffer,
            &self.timestamp_readback_buffer,
        ) {
            encoder.resolve_query_set(query_set, 0..2, resolve, 0);
            encoder.copy_buffer_to_buffer(resolve, 0, readback, 0, (2 * size_of::<u64>()) as u64);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        let _ = self.device.poll(wgpu::MaintainBase::Wait);
        let compute_wall_ms = compute_started.elapsed().as_secs_f32() * 1000.0;

        let mut compute_gpu_ms = compute_wall_ms;
        if self.timestamp_enabled {
            if let Some(readback) = &self.timestamp_readback_buffer {
                let ts_bytes = map_buffer_blocking(&self.device, readback)?;
                let ts = bytemuck::cast_slice::<u8, u64>(&ts_bytes);
                if ts.len() >= 2 {
                    let ticks = ts[1].saturating_sub(ts[0]) as f32;
                    compute_gpu_ms = ticks * self.timestamp_period / 1_000_000.0;
                }
            }
        }

        Ok(GpuStepTimings {
            compute_wall_ms,
            compute_gpu_ms,
            upload_to_gpu_ms: 0.0,
            readback_from_gpu_ms: 0.0,
            step_total_ms: total_started.elapsed().as_secs_f32() * 1000.0,
        })
    }

/// Runs `readback_state` logic.
    pub fn readback_state(&self) -> Result<GpuSolverHostState, String> {
        let species_size = (self.cells * self.gas_count as usize * size_of::<u32>()) as u64;
        let total_density_size = (self.cells * size_of::<f32>()) as u64;
        let velocity_size = (self.cells * size_of::<[f32; 2]>()) as u64;

        let species_staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("species-staging"),
            size: species_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let total_density_staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("total-density-staging"),
            size: total_density_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let velocity_staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("velocity-staging"),
            size: velocity_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("solver-readback-encoder"),
            });
        encoder.copy_buffer_to_buffer(
            &self.species_buffers[self.active_index],
            0,
            &species_staging,
            0,
            species_size,
        );
        encoder.copy_buffer_to_buffer(
            &self.total_density_buffer,
            0,
            &total_density_staging,
            0,
            total_density_size,
        );
        encoder.copy_buffer_to_buffer(
            &self.velocity_buffer,
            0,
            &velocity_staging,
            0,
            velocity_size,
        );
        self.queue.submit(std::iter::once(encoder.finish()));
        let _ = self.device.poll(wgpu::MaintainBase::Wait);

        let species_bytes = map_buffer_blocking(&self.device, &species_staging)?;
        let total_density_bytes = map_buffer_blocking(&self.device, &total_density_staging)?;
        let velocity_bytes = map_buffer_blocking(&self.device, &velocity_staging)?;

        let species = bytemuck::cast_slice::<u8, u32>(&species_bytes).to_vec();
        let total_density = bytemuck::cast_slice::<u8, f32>(&total_density_bytes).to_vec();
        let velocity = bytemuck::cast_slice::<u8, [f32; 2]>(&velocity_bytes).to_vec();

        Ok(GpuSolverHostState {
            gas_count: self.gas_count,
            molecular_masses: self.molecular_masses.clone(),
            species,
            total_density,
            velocity,
            solid_mask: self.solid_mask_host.clone(),
        })
    }

/// Runs `from_cpu_state` logic.
    pub fn from_cpu_state(world: &WorldGrid, gas: &GasField) -> Result<(Self, f32), String> {
        let width = crate::world::grid::WORLD_WIDTH;
        let height = crate::world::grid::WORLD_HEIGHT;
        let mut solver = Self::new_with_gas_count(width, height, gas.gas_count() as u32)?;
        let host_state = gas.to_gpu_host_state(world);
        let upload_ms = solver.upload_state(&host_state)?;
        Ok((solver, upload_ms))
    }

/// Runs `params_from_config` logic.
    pub fn params_from_config(
        config: &super::GasSimulationConfig,
        width: u32,
        height: u32,
        step: u64,
        gas: &GasField,
    ) -> ParamsPod {
        let m0 = gas.molecular_mass(0);
        let m1 = gas.molecular_mass(1);
        let m2 = gas.molecular_mass(2);
        Self::params_from_raw(
            config,
            width,
            height,
            step,
            gas.gas_count() as u32,
            [m0, m1, m2],
        )
    }

/// Runs `params_from_raw` logic.
    pub fn params_from_raw(
        config: &super::GasSimulationConfig,
        width: u32,
        height: u32,
        step: u64,
        gas_count: u32,
        molecular_masses: [f32; 3],
    ) -> ParamsPod {
        ParamsPod {
            width,
            height,
            step: step as u32,
            gas_count: gas_count.max(1),
            thermal_motion_scale: config.thermal_motion_scale,
            _pad0: [0, 0, 0],
            enable_species_relaxation: u32::from(config.enable_species_relaxation),
            enable_lbm_velocity: u32::from(config.enable_lbm_velocity),
            enable_buoyancy: u32::from(config.solver_tuning.enable_buoyancy),
            _pad1: 0,
            tau_even: config.solver_tuning.tau_even,
            tau_odd: config.solver_tuning.tau_odd,
            target_cfl_like_limit: config.solver_tuning.target_cfl_like_limit,
            buoyancy_strength: config.solver_tuning.buoyancy_strength,
            buoyancy_window_radius: config.solver_tuning.buoyancy_window_radius as u32,
            _pad2: 0,
            _pad3: 0,
            _pad4: 0,
            buoyancy_window_sigma: config.solver_tuning.buoyancy_window_sigma,
            buoyancy_gain: config.solver_tuning.buoyancy_gain,
            buoyancy_alpha: config.solver_tuning.buoyancy_alpha,
            buoyancy_force_cap: config.solver_tuning.buoyancy_force_cap,
            reconcile_every_n_steps: config.reconcile_every_n_steps,
            mass_fix_every_n_steps: config.mass_fix_every_n_steps,
            _pad5: 0,
            _pad6: 0,
            mass_fix_error_threshold: config.mass_fix_error_threshold,
            mass_fix_min_residual: config.mass_fix_min_residual,
            molecular_mass_h2: molecular_masses[0],
            molecular_mass_o2: molecular_masses[1],
            molecular_mass_co2: molecular_masses[2],
            _pad8: 0.0,
        }
    }

/// Runs `current_dimensions` logic.
    pub fn current_dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

