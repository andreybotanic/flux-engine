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
