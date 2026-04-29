use std::borrow::Cow;

use bevy::{
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::{RenderAssetUsages, RenderAssets},
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::{binding_types::texture_storage_2d, *},
        renderer::{RenderContext, RenderDevice},
        texture::GpuImage,
        Render, RenderApp, RenderSet,
    },
};

use crate::{
    simulation::{
        gas::{
            seeded_gas_amount, HYDROGEN_GPU_STORAGE_MAX_PARTICLES, HYDROGEN_MAX_VISUAL_PARTICLES,
        },
        SimulationStep,
    },
    world::grid::{is_boundary, WORLD_HEIGHT, WORLD_WIDTH},
};

const SHADER_ASSET_PATH: &str = "shaders/gas_diffusion.wgsl";
const WORKGROUP_SIZE: u32 = 8;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct GasComputeLabel;

#[derive(Resource, Clone, ExtractResource)]
pub struct GasSimulationImages {
    pub texture_a: Handle<Image>,
    pub texture_b: Handle<Image>,
}

#[derive(Resource)]
struct GasBindGroups([BindGroup; 2]);

#[derive(Resource)]
struct GasComputePipeline {
    texture_bind_group_layout: BindGroupLayout,
    pipeline: CachedComputePipelineId,
}

struct GasComputeNode {
    pipeline_ready: bool,
    completed_steps: u64,
    pending_dispatches: u64,
}

impl Default for GasComputeNode {
    fn default() -> Self {
        Self {
            pipeline_ready: false,
            completed_steps: 0,
            pending_dispatches: 0,
        }
    }
}

pub struct GasGpuPlugin;

impl Plugin for GasGpuPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractResourcePlugin::<GasSimulationImages>::default(),
            ExtractResourcePlugin::<SimulationStep>::default(),
        ))
        .add_systems(Startup, setup_simulation_images);

        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(Render, prepare_bind_groups.in_set(RenderSet::PrepareBindGroups));

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(GasComputeLabel, GasComputeNode::default());
        render_graph.add_node_edge(GasComputeLabel, bevy::render::graph::CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<GasComputePipeline>();
    }
}

impl render_graph::Node for GasComputeNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<GasComputePipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        self.pipeline_ready = matches!(
            pipeline_cache.get_compute_pipeline_state(pipeline.pipeline),
            CachedPipelineState::Ok(_)
        );

        if !self.pipeline_ready || !world.contains_resource::<GasBindGroups>() {
            self.pending_dispatches = 0;
            return;
        }

        // Overlay data is synchronized from CPU state in the main world.
        // Keep compute pipeline initialized but skip dispatches for now.
        self.pending_dispatches = 0;
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        if !self.pipeline_ready || self.pending_dispatches == 0 {
            return Ok(());
        }

        let bind_groups = &world.resource::<GasBindGroups>().0;
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<GasComputePipeline>();
        let compute_pipeline = pipeline_cache.get_compute_pipeline(pipeline.pipeline).unwrap();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_pipeline(compute_pipeline);

        let base_step = self.completed_steps - self.pending_dispatches;
        for dispatch_index in 0..self.pending_dispatches {
            let bind_group_index = ((base_step + dispatch_index) as usize) % 2;
            pass.set_bind_group(0, &bind_groups[bind_group_index], &[]);
            pass.dispatch_workgroups(
                WORLD_WIDTH.div_ceil(WORKGROUP_SIZE),
                WORLD_HEIGHT.div_ceil(WORKGROUP_SIZE),
                1,
            );
        }

        Ok(())
    }
}

pub fn setup_simulation_images(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let image_a = images.add(build_seeded_image());
    let image_b = images.add(build_seeded_image());

    commands.insert_resource(GasSimulationImages {
        texture_a: image_a,
        texture_b: image_b,
    });
}

fn build_seeded_image() -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width: WORLD_WIDTH,
            height: WORLD_HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0; 16],
        TextureFormat::Rgba32Float,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;

    for y in 0..WORLD_HEIGHT {
        for x in 0..WORLD_WIDTH {
            let wall = if is_boundary(x, y) { 1.0 } else { 0.0 };
            let particles = seeded_gas_amount(x, y) as f32;
            let storage_linear = (particles / HYDROGEN_GPU_STORAGE_MAX_PARTICLES as f32).clamp(0.0, 1.0);
            let visual_linear = (particles / HYDROGEN_MAX_VISUAL_PARTICLES as f32).clamp(0.0, 1.0);
            let visual = visual_linear.powf(0.35);
            let color = Color::linear_rgba(visual, wall, storage_linear, 1.0);
            let _ = image.set_color_at(x, y, color);
        }
    }

    image
}

fn prepare_bind_groups(
    mut commands: Commands,
    pipeline: Res<GasComputePipeline>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    simulation_images: Res<GasSimulationImages>,
    render_device: Res<RenderDevice>,
) {
    let view_a = gpu_images.get(&simulation_images.texture_a).unwrap();
    let view_b = gpu_images.get(&simulation_images.texture_b).unwrap();

    let bind_group_0 = render_device.create_bind_group(
        None,
        &pipeline.texture_bind_group_layout,
        &BindGroupEntries::sequential((&view_a.texture_view, &view_b.texture_view)),
    );
    let bind_group_1 = render_device.create_bind_group(
        None,
        &pipeline.texture_bind_group_layout,
        &BindGroupEntries::sequential((&view_b.texture_view, &view_a.texture_view)),
    );

    commands.insert_resource(GasBindGroups([bind_group_0, bind_group_1]));
}

impl FromWorld for GasComputePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let texture_bind_group_layout = render_device.create_bind_group_layout(
            "GasSimulationImages",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_storage_2d(TextureFormat::Rgba32Float, StorageTextureAccess::ReadOnly),
                    texture_storage_2d(TextureFormat::Rgba32Float, StorageTextureAccess::WriteOnly),
                ),
            ),
        );
        let shader = world.load_asset(SHADER_ASSET_PATH);
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("Gas diffusion compute shader".into()),
            layout: vec![texture_bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader,
            shader_defs: vec![],
            entry_point: Cow::from("update"),
            zero_initialize_workgroup_memory: false,
        });

        Self {
            texture_bind_group_layout,
            pipeline,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_image_keeps_main_world_usage_for_cpu_updates() {
        let image = build_seeded_image();
        assert!(image.asset_usage.contains(RenderAssetUsages::MAIN_WORLD));
        assert!(image.asset_usage.contains(RenderAssetUsages::RENDER_WORLD));
    }
}
