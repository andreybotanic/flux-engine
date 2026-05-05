#![allow(
    dead_code,
    reason = "ShaderType derive emits internal GPU-layout helpers that are not called directly."
)]

use bevy::{
    prelude::*,
    reflect::TypePath,
    render::{
        mesh::Mesh2d,
        render_resource::{AsBindGroup, ShaderRef, ShaderType},
    },
    sprite::{AlphaMode2d, Material2d, Material2dPlugin, MeshMaterial2d},
};

const PIPE_HIGHLIGHT_SHADER_ASSET_PATH: &str = "shaders/pipe_highlight_material.wgsl";

#[derive(Clone, ShaderType, Debug)]
struct PipeHighlightUniform {
    tint: Vec4,
    params: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
/// Material for the bright `F3` pipe highlight pass.
pub struct PipeHighlightMaterial {
    #[uniform(0)]
    settings: PipeHighlightUniform,
    #[texture(1)]
    #[sampler(2)]
    texture: Handle<Image>,
}

impl PipeHighlightMaterial {
    /// Creates a highlight material that samples the regular pipe sprite.
    pub(crate) fn from_pipe_mask(texture: Handle<Image>) -> Self {
        Self {
            settings: PipeHighlightUniform {
                tint: Vec4::new(1.0, 1.0, 1.0, 1.0),
                params: Vec4::new(1.6, 1.45, 0.18, 0.92),
            },
            texture,
        }
    }
}

impl Material2d for PipeHighlightMaterial {
    fn fragment_shader() -> ShaderRef {
        PIPE_HIGHLIGHT_SHADER_ASSET_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

#[derive(Default)]
/// Plugin that registers the pipe highlight material for `Mesh2d` rendering.
pub(crate) struct PipeHighlightMaterialPlugin;

impl Plugin for PipeHighlightMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<PipeHighlightMaterial>::default());
    }
}

#[derive(Clone)]
/// Shared mesh+material bundle data for `F3` pipe highlighting.
pub(crate) struct PipeHighlightRenderAssets {
    pub quad: Handle<Mesh>,
    pub materials: Vec<Handle<PipeHighlightMaterial>>,
}

/// Spawns one highlight overlay entity for a pipe tile.
pub(crate) fn spawn_pipe_highlight_entity(
    commands: &mut Commands,
    render_assets: &PipeHighlightRenderAssets,
    mask: usize,
    transform: Transform,
    visibility: Visibility,
) -> Entity {
    commands
        .spawn((
            Mesh2d(render_assets.quad.clone()),
            MeshMaterial2d(render_assets.materials[mask].clone()),
            transform,
            visibility,
        ))
        .id()
}
