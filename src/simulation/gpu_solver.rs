use std::{
    borrow::Cow,
    mem::size_of,
    sync::mpsc,
    time::{Duration, Instant},
};

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::simulation::gas::GasField;
use crate::world::grid::WorldGrid;

const SHADER_SOURCE: &str = include_str!("../../assets/shaders/gas_solver.wgsl");

#[derive(Clone, Copy, Debug, Default)]
pub struct GpuStepTimings {
    pub compute_wall_ms: f32,
    pub compute_gpu_ms: f32,
    pub upload_to_gpu_ms: f32,
    pub readback_from_gpu_ms: f32,
    pub step_total_ms: f32,
}

#[derive(Clone, Debug)]
pub struct GpuAdapterInfo {
    pub name: String,
    pub vendor: u32,
    pub device: u32,
    pub backend: String,
    pub driver: String,
    pub driver_info: String,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct ParamsPod {
    width: u32,
    height: u32,
    step: u32,
    gas_count: u32,
    thermal_motion_scale: f32,
    _pad0: [u32; 3],

    enable_species_relaxation: u32,
    enable_lbm_velocity: u32,
    enable_buoyancy: u32,
    _pad1: u32,

    tau_even: f32,
    tau_odd: f32,
    target_cfl_like_limit: f32,
    buoyancy_strength: f32,

    buoyancy_window_radius: u32,
    _pad2: u32,
    _pad3: u32,
    _pad4: u32,

    buoyancy_window_sigma: f32,
    buoyancy_gain: f32,
    buoyancy_alpha: f32,
    buoyancy_force_cap: f32,

    reconcile_every_n_steps: u32,
    mass_fix_every_n_steps: u32,
    _pad5: u32,
    _pad6: u32,

    mass_fix_error_threshold: f32,
    mass_fix_min_residual: f32,
    molecular_mass_h2: f32,
    molecular_mass_o2: f32,
    molecular_mass_co2: f32,
    _pad8: f32,
}

#[derive(Clone)]
pub struct GpuSolverHostState {
    pub gas_count: u32,
    pub molecular_masses: Vec<f32>,
    pub species: Vec<u32>,
    pub total_density: Vec<f32>,
    pub velocity: Vec<[f32; 2]>,
    pub solid_mask: Vec<u32>,
}

struct Pipelines {
    compute_shares: wgpu::ComputePipeline,
    step_discrete_exact: wgpu::ComputePipeline,
}

pub struct GpuGasSolver {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipelines: Pipelines,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_groups: [wgpu::BindGroup; 2],
    params_buffer: wgpu::Buffer,
    species_buffers: [wgpu::Buffer; 2],
    shares_buffer: wgpu::Buffer,
    total_density_buffer: wgpu::Buffer,
    velocity_buffer: wgpu::Buffer,
    solid_mask_buffer: wgpu::Buffer,
    molecular_masses_buffer: wgpu::Buffer,
    active_index: usize,
    width: u32,
    height: u32,
    cells: usize,
    solid_mask_host: Vec<u32>,
    timestamp_period: f32,
    timestamp_enabled: bool,
    timestamp_inside_encoders: bool,
    timestamp_inside_passes: bool,
    timestamp_query_set: Option<wgpu::QuerySet>,
    timestamp_resolve_buffer: Option<wgpu::Buffer>,
    timestamp_readback_buffer: Option<wgpu::Buffer>,
    adapter_info: GpuAdapterInfo,
    gas_count: u32,
    gas_capacity: u32,
    molecular_masses: Vec<f32>,
}

struct GasBuffers {
    species_buffers: [wgpu::Buffer; 2],
    shares_buffer: wgpu::Buffer,
    molecular_masses_buffer: wgpu::Buffer,
}

fn map_buffer_blocking(device: &wgpu::Device, buffer: &wgpu::Buffer) -> Result<Vec<u8>, String> {
    let slice = buffer.slice(..);
    let (tx, rx) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    let _ = device.poll(wgpu::MaintainBase::Wait);
    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(Ok(())) => {
            let data = slice.get_mapped_range().to_vec();
            buffer.unmap();
            Ok(data)
        }
        Ok(Err(err)) => Err(format!("Failed to map buffer: {err}")),
        Err(err) => Err(format!("Timed out waiting buffer map: {err}")),
    }
}

impl GpuGasSolver {
    pub fn adapter_info(&self) -> GpuAdapterInfo {
        self.adapter_info.clone()
    }

    pub fn new(width: u32, height: u32) -> Result<Self, String> {
        Self::create(width, height, 1)
    }

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

    pub fn from_cpu_state(world: &WorldGrid, gas: &GasField) -> Result<(Self, f32), String> {
        let width = crate::world::grid::WORLD_WIDTH;
        let height = crate::world::grid::WORLD_HEIGHT;
        let mut solver = Self::new_with_gas_count(width, height, gas.gas_count() as u32)?;
        let host_state = gas.to_gpu_host_state(world);
        let upload_ms = solver.upload_state(&host_state)?;
        Ok((solver, upload_ms))
    }

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

    pub fn current_dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

fn create_gas_buffers(
    device: &wgpu::Device,
    cells: usize,
    gas_capacity: u32,
) -> Result<GasBuffers, String> {
    if gas_capacity == 0 {
        return Err("gas_capacity must be > 0".to_string());
    }
    let species_count = cells
        .checked_mul(gas_capacity as usize)
        .ok_or_else(|| "species buffer size overflow".to_string())?;
    let shares_count = species_count
        .checked_mul(5)
        .ok_or_else(|| "shares buffer size overflow".to_string())?;
    let species_size = (species_count * size_of::<u32>()) as wgpu::BufferAddress;
    let shares_size = (shares_count * size_of::<u32>()) as wgpu::BufferAddress;
    let masses_size = (gas_capacity as usize * size_of::<f32>()) as wgpu::BufferAddress;

    let species_buffers = [
        create_storage_buffer(device, "species-buffer-a", species_size),
        create_storage_buffer(device, "species-buffer-b", species_size),
    ];
    let shares_buffer = create_storage_buffer(device, "shares-buffer", shares_size);
    let molecular_masses_buffer = create_storage_buffer(device, "molecular-masses", masses_size);

    Ok(GasBuffers {
        species_buffers,
        shares_buffer,
        molecular_masses_buffer,
    })
}

fn create_bind_groups(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    species_buffers: &[wgpu::Buffer; 2],
    shares_buffer: &wgpu::Buffer,
    total_density_buffer: &wgpu::Buffer,
    velocity_buffer: &wgpu::Buffer,
    solid_mask_buffer: &wgpu::Buffer,
    params_buffer: &wgpu::Buffer,
    molecular_masses_buffer: &wgpu::Buffer,
) -> [wgpu::BindGroup; 2] {
    let make = |read_idx: usize, write_idx: usize| {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gas-solver-bind-group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: species_buffers[read_idx].as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: species_buffers[write_idx].as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: shares_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: total_density_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: velocity_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: solid_mask_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: molecular_masses_buffer.as_entire_binding(),
                },
            ],
        })
    };
    [make(0, 1), make(1, 0)]
}

fn storage_entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn create_storage_buffer(
    device: &wgpu::Device,
    label: &str,
    size: wgpu::BufferAddress,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    })
}

fn dispatch_pipeline(
    encoder: &mut wgpu::CommandEncoder,
    pipeline: &wgpu::ComputePipeline,
    bind_group: &wgpu::BindGroup,
    workgroups_x: u32,
    timestamp_writes: Option<wgpu::ComputePassTimestampWrites<'_>>,
) {
    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: None,
        timestamp_writes,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bind_group, &[]);
    pass.dispatch_workgroups(workgroups_x.max(1), 1, 1);
}
