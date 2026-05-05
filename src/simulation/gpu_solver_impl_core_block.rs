impl GpuGasSolver {
/// Runs `adapter_info` logic.
    pub fn adapter_info(&self) -> GpuAdapterInfo {
        self.adapter_info.clone()
    }

/// Runs `new` logic.
    pub fn new(width: u32, height: u32) -> Result<Self, String> {
        Self::create(width, height, 1)
    }

/// Runs `new_with_gas_count` logic.
    pub fn new_with_gas_count(width: u32, height: u32, gas_count: u32) -> Result<Self, String> {
        Self::create(width, height, gas_count.max(1))
    }

    fn create(width: u32, height: u32, gas_capacity: u32) -> Result<Self, String> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        }))
        .ok_or_else(|| "request_adapter returned None".to_string())?;
        let raw_info = adapter.get_info();
        let adapter_info = GpuAdapterInfo {
            name: raw_info.name,
            vendor: raw_info.vendor,
            device: raw_info.device,
            backend: format!("{:?}", raw_info.backend),
            driver: raw_info.driver,
            driver_info: raw_info.driver_info,
        };

        let features = adapter.features();
        let timestamp_enabled = features.contains(wgpu::Features::TIMESTAMP_QUERY);
        let timestamp_inside_encoders =
            features.contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS);
        let timestamp_inside_passes =
            features.contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES);
        let mut required_features = wgpu::Features::empty();
        if timestamp_enabled {
            required_features |= wgpu::Features::TIMESTAMP_QUERY;
        }
        if timestamp_inside_encoders {
            required_features |= wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
        }
        if timestamp_inside_passes {
            required_features |= wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES;
        }

        let limits = adapter.limits();
        if limits.max_storage_buffers_per_shader_stage < 7 {
            return Err(format!(
                "GPU limit max_storage_buffers_per_shader_stage={} is too low for solver (need >=7)",
                limits.max_storage_buffers_per_shader_stage
            ));
        }
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("flux-engine-gpu-solver"),
                required_features,
                required_limits: limits,
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .map_err(|e| format!("request_device failed: {e}"))?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gas-solver-wgsl"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER_SOURCE)),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gas-solver-bind-group-layout"),
            entries: &[
                storage_entry(0, true),
                storage_entry(1, false),
                storage_entry(2, false),
                storage_entry(3, false),
                storage_entry(4, false),
                storage_entry(5, true),
                uniform_entry(6),
                storage_entry(7, true),
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gas-solver-pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let build_pipeline = |entry: &str| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry),
                cache: None,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            })
        };

        let pipelines = Pipelines {
            compute_shares: build_pipeline("compute_shares"),
            step_discrete_exact: build_pipeline("step_discrete_exact"),
        };

        let cells = usize::try_from(width)
            .ok()
            .and_then(|w| usize::try_from(height).ok().map(|h| w * h))
            .ok_or("Failed to compute cells count")?;

        let total_density_size = (cells * size_of::<f32>()) as wgpu::BufferAddress;
        let velocity_size = (cells * size_of::<[f32; 2]>()) as wgpu::BufferAddress;
        let solid_mask_size = (cells * size_of::<u32>()) as wgpu::BufferAddress;

        let total_density_buffer =
            create_storage_buffer(&device, "total-density-buffer", total_density_size);
        let velocity_buffer = create_storage_buffer(&device, "velocity-buffer", velocity_size);
        let solid_mask_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("solid-mask-buffer"),
            size: solid_mask_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("solver-params-buffer"),
            contents: bytemuck::bytes_of(&ParamsPod::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let gas_buffers = create_gas_buffers(&device, cells, gas_capacity)?;
        let bind_groups = create_bind_groups(
            &device,
            &bind_group_layout,
            &gas_buffers.species_buffers,
            &gas_buffers.shares_buffer,
            &total_density_buffer,
            &velocity_buffer,
            &solid_mask_buffer,
            &params_buffer,
            &gas_buffers.molecular_masses_buffer,
        );

        let (timestamp_query_set, timestamp_resolve_buffer, timestamp_readback_buffer) =
            if timestamp_enabled {
                let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
                    label: Some("solver-timestamp-query-set"),
                    ty: wgpu::QueryType::Timestamp,
                    count: 2,
                });
                let resolve_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("solver-timestamp-resolve"),
                    size: (2 * size_of::<u64>()) as wgpu::BufferAddress,
                    usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                });
                let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("solver-timestamp-readback"),
                    size: (2 * size_of::<u64>()) as wgpu::BufferAddress,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                (Some(query_set), Some(resolve_buffer), Some(readback_buffer))
            } else {
                (None, None, None)
            };

        let timestamp_period = queue.get_timestamp_period();

        Ok(Self {
            device,
            queue,
            pipelines,
            bind_group_layout,
            bind_groups,
            params_buffer,
            species_buffers: gas_buffers.species_buffers,
            shares_buffer: gas_buffers.shares_buffer,
            total_density_buffer,
            velocity_buffer,
            solid_mask_buffer,
            molecular_masses_buffer: gas_buffers.molecular_masses_buffer,
            active_index: 0,
            width,
            height,
            cells,
            solid_mask_host: vec![0; cells],
            timestamp_period,
            timestamp_enabled,
            timestamp_inside_encoders,
            timestamp_inside_passes,
            timestamp_query_set,
            timestamp_resolve_buffer,
            timestamp_readback_buffer,
            adapter_info,
            gas_count: gas_capacity,
            gas_capacity,
            molecular_masses: vec![1.0; gas_capacity as usize],
        })
    }

    fn ensure_gas_capacity(&mut self, gas_count: u32) -> Result<(), String> {
        let need = gas_count.max(1);
        if need <= self.gas_capacity {
            return Ok(());
        }
        let gas_buffers = create_gas_buffers(&self.device, self.cells, need)?;
        let bind_groups = create_bind_groups(
            &self.device,
            &self.bind_group_layout,
            &gas_buffers.species_buffers,
            &gas_buffers.shares_buffer,
            &self.total_density_buffer,
            &self.velocity_buffer,
            &self.solid_mask_buffer,
            &self.params_buffer,
            &gas_buffers.molecular_masses_buffer,
        );
        self.species_buffers = gas_buffers.species_buffers;
        self.shares_buffer = gas_buffers.shares_buffer;
        self.molecular_masses_buffer = gas_buffers.molecular_masses_buffer;
        self.bind_groups = bind_groups;
        self.active_index = 0;
        self.gas_capacity = need;
        Ok(())
    }

/// Runs `upload_state` logic.
    pub fn upload_state(&mut self, state: &GpuSolverHostState) -> Result<f32, String> {
        let started = Instant::now();
        let gas_count = state.gas_count.max(1);
        let expected_species = self
            .cells
            .checked_mul(gas_count as usize)
            .ok_or_else(|| "Species count overflow".to_string())?;
        if state.species.len() != expected_species {
            return Err(format!(
                "GPU upload species length mismatch: got {}, expected {}",
                state.species.len(),
                expected_species
            ));
        }
        if state.total_density.len() != self.cells {
            return Err(format!(
                "GPU upload total_density length mismatch: got {}, expected {}",
                state.total_density.len(),
                self.cells
            ));
        }
        if state.velocity.len() != self.cells {
            return Err(format!(
                "GPU upload velocity length mismatch: got {}, expected {}",
                state.velocity.len(),
                self.cells
            ));
        }
        if state.solid_mask.len() != self.cells {
            return Err(format!(
                "GPU upload solid_mask length mismatch: got {}, expected {}",
                state.solid_mask.len(),
                self.cells
            ));
        }

        self.ensure_gas_capacity(gas_count)?;

        let mut masses = vec![1.0f32; gas_count as usize];
        for (i, mass) in state.molecular_masses.iter().copied().enumerate() {
            if i >= masses.len() {
                break;
            }
            masses[i] = mass;
        }

        self.queue.write_buffer(
            &self.species_buffers[self.active_index],
            0,
            bytemuck::cast_slice(&state.species),
        );
        self.queue.write_buffer(
            &self.species_buffers[1 - self.active_index],
            0,
            bytemuck::cast_slice(&state.species),
        );
        self.queue.write_buffer(
            &self.total_density_buffer,
            0,
            bytemuck::cast_slice(&state.total_density),
        );
        self.queue.write_buffer(
            &self.velocity_buffer,
            0,
            bytemuck::cast_slice(&state.velocity),
        );
        self.queue.write_buffer(
            &self.solid_mask_buffer,
            0,
            bytemuck::cast_slice(&state.solid_mask),
        );
        self.queue.write_buffer(
            &self.molecular_masses_buffer,
            0,
            bytemuck::cast_slice(&masses),
        );

        self.solid_mask_host = state.solid_mask.clone();
        self.gas_count = gas_count;
        self.molecular_masses = masses;

        Ok(started.elapsed().as_secs_f32() * 1000.0)
    }

}
