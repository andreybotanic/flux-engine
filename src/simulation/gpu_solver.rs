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
/// Stores `GpuStepTimings` state.
pub struct GpuStepTimings {
    pub compute_wall_ms: f32,
    pub compute_gpu_ms: f32,
    pub upload_to_gpu_ms: f32,
    pub readback_from_gpu_ms: f32,
    pub step_total_ms: f32,
}

#[derive(Clone, Debug)]
/// Stores `GpuAdapterInfo` state.
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
/// Stores `ParamsPod` state.
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
/// Stores `GpuSolverHostState` state.
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

/// Stores `GpuGasSolver` state.
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

include!("gpu_solver_impl_core_block.rs");
include!("gpu_solver_impl_exec_block.rs");
include!("gpu_solver_helpers_block.rs");
