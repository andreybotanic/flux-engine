use std::collections::HashMap;

use bevy::{
    prelude::*,
    render::render_asset::RenderAssetUsages,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    sprite::SpriteImageMode,
};

use crate::{
    config::{GasMainViewVisualConfig, StructureVisualConfigMap},
    plugins::{
        ContentId, ContentRegistry, PluginOverlayGraphStore, PluginRuntimeRegistry,
        RuntimeOverlayDescriptor,
    },
    render::{
        pipe_highlight_material::{
            spawn_pipe_highlight_entity, spawn_pipe_highlight_material_entity,
        },
        world_view::{GasSimulationImages, OverlayMode, WorldVisualAssets},
    },
    save::WorldLoadState,
    simulation::SimulationStep,
    world::{
        grid::{cell_center, linear_index, world_origin, CellKind, WorldGrid, CELL_SIZE},
        structures::{PlacedStructure, PlacedStructureMap},
    },
};

const OVERLAY_ENTITY_BASE_Z: f32 = 1.10;
const OVERLAY_LAYER_STEP_Z: f32 = 0.02;
const DEFAULT_WHITE_IMAGE_ID: &str = "flux.default.overlay.image.white";
const DEFAULT_VENT_ICON_IMAGE_ID: &str = "flux.default.overlay.image.vent_icon";
const DEFAULT_GAS_IN_ICON_IMAGE_ID: &str = "flux.default.overlay.image.gas_in_icon";
const DEFAULT_GAS_OUT_ICON_IMAGE_ID: &str = "flux.default.overlay.image.gas_out_icon";
const DEFAULT_PIPE_HIGHLIGHT_MATERIAL_ID: &str = "flux.default.overlay.material.pipe_highlight";

#[derive(Resource, Clone)]
pub(crate) struct OverlayGraphAssets {
    white_square: Handle<Image>,
}

#[derive(Resource, Default)]
pub(crate) struct OverlayGraphSceneState {
    pub active_overlay: Option<ContentId>,
    overlay_entities: Vec<Entity>,
    dynamic_images: HashMap<String, Handle<Image>>,
}

impl OverlayGraphSceneState {
    pub(crate) fn renders_overlay(&self, overlay_id: &str) -> bool {
        self.active_overlay
            .as_ref()
            .map(|id| id.as_str() == overlay_id)
            .unwrap_or(false)
    }
}

#[derive(Clone)]
enum LayerPlan {
    Entities {
        selector: flux_plugin_sdk::OverlaySelectorExpr,
        style: flux_plugin_sdk::OverlayEntityStyle,
        material: Option<flux_plugin_sdk::OverlayMaterialId>,
    },
    FreeGas {
        alpha: f32,
    },
    Images {
        instances: Vec<flux_plugin_sdk::OverlayImageInstance>,
    },
}

pub(crate) fn setup_overlay_graph_assets(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    let mut white = Image::new_fill(
        Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[255, 255, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    white.texture_descriptor.usage = TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING;
    commands.insert_resource(OverlayGraphAssets {
        white_square: images.add(white),
    });
}

pub(crate) fn sync_overlay_graph_visuals(
    mut commands: Commands,
    mode_resources: (Res<OverlayMode>, Res<WorldLoadState>),
    overlay_resources: (
        Res<PluginRuntimeRegistry>,
        Res<ContentRegistry>,
        Res<OverlayGraphAssets>,
        Res<PluginOverlayGraphStore>,
    ),
    world_resources: (
        Res<GasSimulationImages>,
        Res<SimulationStep>,
        Res<WorldGrid>,
        Res<PlacedStructureMap>,
        Res<WorldVisualAssets>,
        Res<StructureVisualConfigMap>,
        Res<GasMainViewVisualConfig>,
        Res<AssetServer>,
    ),
    render_resources: (ResMut<Assets<Image>>, ResMut<OverlayGraphSceneState>),
) {
    let (overlay_mode, world_load_state) = mode_resources;
    let (runtime_registry, content_registry, overlay_assets, overlay_graph_store) =
        overlay_resources;
    let (
        simulation_images,
        simulation_step,
        world,
        structures,
        world_visuals,
        structure_visuals,
        gas_main_visual,
        asset_server,
    ) = world_resources;
    let (mut images, mut scene_state) = render_resources;

    clear_overlay_entities(&mut commands, &mut scene_state);
    scene_state.active_overlay = None;

    if !world_load_state.has_world {
        return;
    }

    let OverlayMode::Plugin(raw_overlay_id) = *overlay_mode else {
        return;
    };
    let Ok(overlay_id) = ContentId::parse(raw_overlay_id) else {
        return;
    };
    let Some(descriptor) = runtime_registry.overlays().get(&overlay_id) else {
        return;
    };

    let graph = descriptor
        .graph
        .clone()
        .or_else(|| overlay_graph_store.graph.clone());

    if let Some(graph) = graph {
        if graph_references_known_materials(&graph, &runtime_registry) {
            if let Ok(order) = graph.execution_order() {
                if let Some(layers) = evaluate_graph(&graph, &order) {
                    scene_state.active_overlay = Some(overlay_id);
                    render_layers(
                        &mut commands,
                        &content_registry,
                        &overlay_assets,
                        &simulation_images,
                        simulation_step.0,
                        &world,
                        &structures,
                        &world_visuals,
                        &structure_visuals,
                        &gas_main_visual,
                        &asset_server,
                        &mut images,
                        &mut scene_state,
                        descriptor,
                        &layers,
                    );
                    return;
                }
            }
        }
    }
}

fn evaluate_graph(
    graph: &flux_plugin_sdk::OverlayGraph,
    order: &[flux_plugin_sdk::OverlayNodeId],
) -> Option<Vec<LayerPlan>> {
    let nodes = graph
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node))
        .collect::<HashMap<_, _>>();
    let mut outputs: HashMap<flux_plugin_sdk::OverlayNodeId, Vec<LayerPlan>> = HashMap::new();

    for node_id in order {
        let node = nodes.get(node_id)?;
        let layers = match &node.kind {
            flux_plugin_sdk::OverlayNodeKind::RenderEntities(render) => vec![LayerPlan::Entities {
                selector: render.selector.clone(),
                style: render.style.clone(),
                material: None,
            }],
            flux_plugin_sdk::OverlayNodeKind::RenderFreeGas(render) => {
                vec![LayerPlan::FreeGas {
                    alpha: render.alpha,
                }]
            }
            flux_plugin_sdk::OverlayNodeKind::RenderImage(render) => vec![LayerPlan::Images {
                instances: render.instances.clone(),
            }],
            flux_plugin_sdk::OverlayNodeKind::Blend(_) => {
                let mut combined = Vec::new();
                for dependency in &node.depends_on {
                    combined.extend(outputs.get(dependency)?.clone());
                }
                combined
            }
            flux_plugin_sdk::OverlayNodeKind::Material(render) => {
                let mut combined = Vec::new();
                for dependency in &node.depends_on {
                    for layer in outputs.get(dependency)?.clone() {
                        combined.push(match layer {
                            LayerPlan::Entities {
                                selector,
                                style,
                                material: _,
                            } => LayerPlan::Entities {
                                selector,
                                style,
                                material: Some(render.material_id.clone()),
                            },
                            other => other,
                        });
                    }
                }
                combined
            }
        };
        outputs.insert(node.id.clone(), layers);
    }

    outputs.remove(&graph.output)
}

fn graph_references_known_materials(
    graph: &flux_plugin_sdk::OverlayGraph,
    runtime_registry: &PluginRuntimeRegistry,
) -> bool {
    for node in &graph.nodes {
        let flux_plugin_sdk::OverlayNodeKind::Material(material) = &node.kind else {
            continue;
        };
        if material.material_id.as_str() == DEFAULT_PIPE_HIGHLIGHT_MATERIAL_ID {
            continue;
        }
        let Ok(material_id) = ContentId::parse(material.material_id.as_str()) else {
            return false;
        };
        if !runtime_registry
            .overlay_materials()
            .contains_key(&material_id)
        {
            return false;
        }
    }
    true
}

#[allow(clippy::too_many_arguments)]
fn render_layers(
    commands: &mut Commands,
    content_registry: &ContentRegistry,
    overlay_assets: &OverlayGraphAssets,
    simulation_images: &GasSimulationImages,
    simulation_step: u64,
    world: &WorldGrid,
    structures: &PlacedStructureMap,
    world_visuals: &WorldVisualAssets,
    structure_visuals: &StructureVisualConfigMap,
    gas_main_visual: &GasMainViewVisualConfig,
    asset_server: &AssetServer,
    images: &mut Assets<Image>,
    scene_state: &mut OverlayGraphSceneState,
    descriptor: &RuntimeOverlayDescriptor,
    layers: &[LayerPlan],
) {
    let structure_draw_ranks = build_structure_draw_ranks(structures, structure_visuals);
    for (layer_index, layer) in layers.iter().enumerate() {
        match layer {
            LayerPlan::Entities {
                selector,
                style,
                material,
            } => render_entity_layer(
                commands,
                content_registry,
                world,
                structures,
                world_visuals,
                structure_visuals,
                asset_server,
                scene_state,
                layer_index,
                selector,
                style,
                material.as_ref(),
                &structure_draw_ranks,
            ),
            LayerPlan::FreeGas { alpha } => render_free_gas_layer(
                commands,
                simulation_images,
                simulation_step,
                gas_main_visual.alpha * *alpha,
                scene_state,
                layer_index,
            ),
            LayerPlan::Images { instances } => render_image_layer(
                commands,
                overlay_assets,
                world_visuals,
                descriptor,
                asset_server,
                images,
                scene_state,
                layer_index,
                instances,
            ),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_entity_layer(
    commands: &mut Commands,
    content_registry: &ContentRegistry,
    world: &WorldGrid,
    structures: &PlacedStructureMap,
    world_visuals: &WorldVisualAssets,
    structure_visuals: &StructureVisualConfigMap,
    asset_server: &AssetServer,
    scene_state: &mut OverlayGraphSceneState,
    layer_index: usize,
    selector: &flux_plugin_sdk::OverlaySelectorExpr,
    style: &flux_plugin_sdk::OverlayEntityStyle,
    material: Option<&flux_plugin_sdk::OverlayMaterialId>,
    structure_draw_ranks: &HashMap<crate::world::structures::PlacedStructureId, usize>,
) {
    for y in 0..crate::world::grid::WORLD_HEIGHT {
        for x in 0..crate::world::grid::WORLD_WIDTH {
            let CellKind::Solid(cell_material) = world.cell(x, y) else {
                continue;
            };
            let Some(descriptor) = content_registry.cell_by_material(cell_material) else {
                continue;
            };
            if !content_registry.cell_matches_selector(&descriptor.id, selector) {
                continue;
            }
            let Some(image) = resolve_entity_sprite_path(&descriptor.sprite, style) else {
                continue;
            };
            let entity = commands
                .spawn((
                    Sprite {
                        image: asset_server.load(image),
                        custom_size: Some(Vec2::splat(CELL_SIZE)),
                        color: vec4_to_color(style.tint.unwrap_or(Vec4::ONE)),
                        ..default()
                    },
                    Transform::from_translation(cell_center(x, y).extend(layer_subject_z(
                        layer_index,
                        descriptor.visual.draw_priority,
                        linear_index(x, y),
                    ))),
                ))
                .id();
            scene_state.overlay_entities.push(entity);
        }
    }

    for structure in structures.iter() {
        let Some(descriptor) = content_registry.structure_by_kind(structure.kind) else {
            continue;
        };
        if !content_registry.structure_matches_selector(&descriptor.id, selector) {
            continue;
        }
        let image_handle = if crate::plugins::default_plugin::is_pipe_structure(structure.kind)
            && style.sprite_override.is_none()
        {
            let mask = pipe_connection_mask(structures, structure.origin) as usize;
            world_visuals.pipe_masks.get(mask).cloned()
        } else {
            let Some(image) = resolve_entity_sprite_path(&descriptor.sprite, style) else {
                continue;
            };
            Some(asset_server.load(image))
        };
        let Some(image_handle) = image_handle else {
            continue;
        };
        let size_in_cells = crate::world::structures::structure_sprite_size_in_cells(
            structure.kind,
            structure.rotation,
            structure_visuals,
        );
        let draw_rank = *structure_draw_ranks.get(&structure.id).unwrap_or(&0);
        let entity = commands
            .spawn((
                Sprite {
                    image: image_handle,
                    custom_size: Some(size_in_world(size_in_cells)),
                    color: vec4_to_color(style.tint.unwrap_or(Vec4::ONE)),
                    image_mode: SpriteImageMode::Auto,
                    ..default()
                },
                structure_transform(
                    structure,
                    layer_subject_z(layer_index, descriptor.visual.draw_priority, draw_rank),
                ),
            ))
            .id();
        scene_state.overlay_entities.push(entity);

        if material
            .map(|material_id| material_id.as_str() == DEFAULT_PIPE_HIGHLIGHT_MATERIAL_ID)
            .unwrap_or(false)
        {
            let z =
                layer_subject_z(layer_index, descriptor.visual.draw_priority, draw_rank) + 0.0005;
            if crate::plugins::default_plugin::is_pipe_structure(structure.kind) {
                let mask = pipe_connection_mask(structures, structure.origin) as usize;
                let highlight = spawn_pipe_highlight_entity(
                    commands,
                    &world_visuals.pipe_highlight,
                    mask,
                    Transform::from_translation(
                        cell_center(structure.origin.x, structure.origin.y).extend(z),
                    ),
                    Visibility::Visible,
                );
                scene_state.overlay_entities.push(highlight);
            } else if crate::plugins::default_plugin::is_vent_structure(structure.kind) {
                let highlight = spawn_pipe_highlight_material_entity(
                    commands,
                    &world_visuals.pipe_highlight,
                    world_visuals.pipe_highlight.vent_material.clone(),
                    Transform::from_translation(
                        cell_center(structure.origin.x, structure.origin.y).extend(z),
                    ),
                    Visibility::Visible,
                );
                scene_state.overlay_entities.push(highlight);
            } else if crate::plugins::default_plugin::is_gas_pipe_bridge_structure(structure.kind) {
                let size_in_cells = crate::world::structures::structure_sprite_size_in_cells(
                    structure.kind,
                    structure.rotation,
                    structure_visuals,
                );
                let mut transform = structure_transform(structure, z);
                transform.scale = Vec3::new(
                    size_in_cells.x.max(1) as f32,
                    size_in_cells.y.max(1) as f32,
                    1.0,
                );
                let highlight = spawn_pipe_highlight_material_entity(
                    commands,
                    &world_visuals.pipe_highlight,
                    world_visuals.pipe_highlight.bridge_material.clone(),
                    transform,
                    Visibility::Visible,
                );
                scene_state.overlay_entities.push(highlight);
            }
        }
    }
}

fn render_free_gas_layer(
    commands: &mut Commands,
    simulation_images: &GasSimulationImages,
    simulation_step: u64,
    alpha: f32,
    scene_state: &mut OverlayGraphSceneState,
    layer_index: usize,
) {
    let texture = if simulation_step % 2 == 0 {
        simulation_images.texture_f1_a.clone()
    } else {
        simulation_images.texture_f1_b.clone()
    };
    let entity = commands
        .spawn((
            Sprite {
                image: texture,
                custom_size: Some(crate::world::grid::world_dimensions()),
                color: Color::srgba(1.0, 1.0, 1.0, alpha.clamp(0.0, 1.0)),
                ..default()
            },
            Transform::from_xyz(
                0.0,
                0.0,
                OVERLAY_ENTITY_BASE_Z + layer_index as f32 * OVERLAY_LAYER_STEP_Z,
            ),
        ))
        .id();
    scene_state.overlay_entities.push(entity);
}

#[allow(clippy::too_many_arguments)]
fn render_image_layer(
    commands: &mut Commands,
    overlay_assets: &OverlayGraphAssets,
    world_visuals: &WorldVisualAssets,
    descriptor: &RuntimeOverlayDescriptor,
    asset_server: &AssetServer,
    images: &mut Assets<Image>,
    scene_state: &mut OverlayGraphSceneState,
    layer_index: usize,
    instances: &[flux_plugin_sdk::OverlayImageInstance],
) {
    for (instance_index, instance) in instances.iter().enumerate() {
        let image = resolve_overlay_image_handle(
            overlay_assets,
            world_visuals,
            descriptor,
            asset_server,
            images,
            scene_state,
            layer_index,
            instance_index,
            &instance.image,
        );
        let Some(image) = image else {
            continue;
        };
        let (translation, size, rotation) = overlay_instance_transform(&instance.placement);
        let icon_z_bonus = match &instance.image {
            flux_plugin_sdk::OverlayImageSource::Asset(id)
                if id.as_str() == DEFAULT_VENT_ICON_IMAGE_ID
                    || id.as_str() == DEFAULT_GAS_IN_ICON_IMAGE_ID
                    || id.as_str() == DEFAULT_GAS_OUT_ICON_IMAGE_ID =>
            {
                0.0005
            }
            _ => 0.0,
        };
        let entity = commands
            .spawn((
                Sprite {
                    image,
                    custom_size: Some(size),
                    color: vec4_to_color(instance.tint),
                    ..default()
                },
                Transform {
                    translation: translation.extend(
                        OVERLAY_ENTITY_BASE_Z
                            + layer_index as f32 * OVERLAY_LAYER_STEP_Z
                            + instance_index as f32 * 0.00000001
                            + icon_z_bonus,
                    ),
                    rotation: Quat::from_rotation_z(rotation),
                    ..default()
                },
            ))
            .id();
        scene_state.overlay_entities.push(entity);
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_overlay_image_handle(
    overlay_assets: &OverlayGraphAssets,
    world_visuals: &WorldVisualAssets,
    descriptor: &RuntimeOverlayDescriptor,
    asset_server: &AssetServer,
    images: &mut Assets<Image>,
    scene_state: &mut OverlayGraphSceneState,
    layer_index: usize,
    instance_index: usize,
    image: &flux_plugin_sdk::OverlayImageSource,
) -> Option<Handle<Image>> {
    match image {
        flux_plugin_sdk::OverlayImageSource::Asset(id) if id.as_str() == DEFAULT_WHITE_IMAGE_ID => {
            Some(overlay_assets.white_square.clone())
        }
        flux_plugin_sdk::OverlayImageSource::Asset(id)
            if id.as_str() == DEFAULT_VENT_ICON_IMAGE_ID =>
        {
            Some(world_visuals.port_overlay_bidir.clone())
        }
        flux_plugin_sdk::OverlayImageSource::Asset(id)
            if id.as_str() == DEFAULT_GAS_IN_ICON_IMAGE_ID =>
        {
            Some(world_visuals.port_overlay_in.clone())
        }
        flux_plugin_sdk::OverlayImageSource::Asset(id)
            if id.as_str() == DEFAULT_GAS_OUT_ICON_IMAGE_ID =>
        {
            Some(world_visuals.port_overlay_out.clone())
        }
        flux_plugin_sdk::OverlayImageSource::Asset(id) => {
            let _ = descriptor;
            Some(asset_server.load(id.as_str().to_string()))
        }
        flux_plugin_sdk::OverlayImageSource::Rgba8 { size_px, rgba8 } => {
            let key = format!("{}:{layer_index}:{instance_index}", descriptor.id.as_str());
            let handle = scene_state.dynamic_images.entry(key).or_insert_with(|| {
                let mut image = Image::new(
                    Extent3d {
                        width: size_px.x,
                        height: size_px.y,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    rgba8.clone(),
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
                );
                image.texture_descriptor.usage =
                    TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING;
                images.add(image)
            });
            let handle_clone = handle.clone();
            if let Some(dynamic_image) = images.get_mut(&handle_clone) {
                dynamic_image.resize(Extent3d {
                    width: size_px.x,
                    height: size_px.y,
                    depth_or_array_layers: 1,
                });
                if let Some(data) = dynamic_image.data.as_mut() {
                    data.clear();
                    data.extend_from_slice(rgba8);
                }
            }
            Some(handle_clone)
        }
    }
}

fn resolve_entity_sprite_path(
    sprite: &crate::plugins::SpriteMetadata,
    style: &flux_plugin_sdk::OverlayEntityStyle,
) -> Option<String> {
    match style.sprite_override.as_ref() {
        Some(flux_plugin_sdk::OverlayEntitySpriteOverride::OverlayVariant) => sprite
            .overlay_path
            .clone()
            .or_else(|| Some(sprite.image_path.clone())),
        Some(flux_plugin_sdk::OverlayEntitySpriteOverride::Silhouette) => sprite
            .silhouette_path
            .clone()
            .or_else(|| Some(sprite.image_path.clone())),
        Some(flux_plugin_sdk::OverlayEntitySpriteOverride::Asset(asset)) => {
            Some(asset.as_str().to_string())
        }
        None => Some(sprite.image_path.clone()),
    }
}

fn overlay_instance_transform(placement: &flux_plugin_sdk::OverlayPlacement) -> (Vec2, Vec2, f32) {
    match placement {
        flux_plugin_sdk::OverlayPlacement::CellLocal {
            cell,
            anchor,
            offset_in_cell,
            size_in_cell,
            rotation,
        } => (
            world_origin()
                + Vec2::new(
                    (cell.x as f32 + anchor.x + offset_in_cell.x) * CELL_SIZE,
                    (cell.y as f32 + anchor.y + offset_in_cell.y) * CELL_SIZE,
                ),
            *size_in_cell * CELL_SIZE,
            *rotation,
        ),
        flux_plugin_sdk::OverlayPlacement::GridLocal {
            position_in_grid,
            size_in_grid,
            rotation,
            origin,
        } => (
            world_origin()
                + (*position_in_grid + (*size_in_grid * (Vec2::splat(0.5) - *origin))) * CELL_SIZE,
            *size_in_grid * CELL_SIZE,
            *rotation,
        ),
    }
}

fn clear_overlay_entities(commands: &mut Commands, state: &mut OverlayGraphSceneState) {
    for entity in state.overlay_entities.drain(..) {
        commands.entity(entity).despawn();
    }
}

fn build_structure_draw_ranks(
    structures: &PlacedStructureMap,
    structure_visuals: &StructureVisualConfigMap,
) -> HashMap<crate::world::structures::PlacedStructureId, usize> {
    let mut ordered = structures.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|structure| {
        (
            structure_visuals.get(structure.kind).draw_priority,
            structure.id.0,
        )
    });
    ordered
        .into_iter()
        .enumerate()
        .map(|(rank, structure)| (structure.id, rank))
        .collect()
}

fn structure_transform(structure: &PlacedStructure, z: f32) -> Transform {
    let mut transform = Transform::from_translation(structure_visual_center(structure).extend(z));
    apply_state_sprite_transform(
        &mut transform,
        crate::plugins::default_plugin::structure_state_sprite_transform(
            structure.kind,
            structure.state,
        ),
    );
    transform
}

fn structure_visual_center(structure: &PlacedStructure) -> Vec2 {
    let descriptor = structure.descriptor();
    let Some((min, max)) = descriptor.local_bounds() else {
        return cell_center(structure.origin.x, structure.origin.y);
    };
    let local_center = Vec2::new((min.x + max.x) as f32 * 0.5, (min.y + max.y) as f32 * 0.5);
    cell_center(structure.origin.x, structure.origin.y) + local_center * CELL_SIZE
}

fn apply_state_sprite_transform(
    transform: &mut Transform,
    state_transform: flux_plugin_sdk::EntitySpriteTransform,
) {
    match state_transform {
        flux_plugin_sdk::EntitySpriteTransform::None => {}
        flux_plugin_sdk::EntitySpriteTransform::Rot90 => {
            transform.rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
        }
        flux_plugin_sdk::EntitySpriteTransform::Rot180 => {
            transform.rotation = Quat::from_rotation_z(std::f32::consts::PI);
        }
        flux_plugin_sdk::EntitySpriteTransform::Rot270 => {
            transform.rotation = Quat::from_rotation_z(std::f32::consts::PI * 1.5);
        }
        flux_plugin_sdk::EntitySpriteTransform::FlipX => {
            transform.scale.x *= -1.0;
        }
        flux_plugin_sdk::EntitySpriteTransform::FlipY => {
            transform.scale.y *= -1.0;
        }
    }
}

fn size_in_world(size_in_cells: UVec2) -> Vec2 {
    Vec2::new(
        size_in_cells.x.max(1) as f32 * CELL_SIZE,
        size_in_cells.y.max(1) as f32 * CELL_SIZE,
    )
}

fn layer_subject_z(layer_index: usize, draw_priority: i32, draw_rank: usize) -> f32 {
    OVERLAY_ENTITY_BASE_Z
        + layer_index as f32 * OVERLAY_LAYER_STEP_Z
        + local_appearance_z(draw_priority, draw_rank)
}

fn pipe_connection_mask(structures: &PlacedStructureMap, cell: UVec2) -> u8 {
    let mut mask = 0u8;
    if cell.y > 0 {
        let neighbor = UVec2::new(cell.x, cell.y - 1);
        if structures.has_pipe_at(neighbor.x, neighbor.y) && !structures.is_pipe_cut(cell, neighbor)
        {
            mask |= 0b0001;
        }
    }
    if cell.x + 1 < crate::world::grid::WORLD_WIDTH {
        let neighbor = UVec2::new(cell.x + 1, cell.y);
        if structures.has_pipe_at(neighbor.x, neighbor.y) && !structures.is_pipe_cut(cell, neighbor)
        {
            mask |= 0b0010;
        }
    }
    if cell.y + 1 < crate::world::grid::WORLD_HEIGHT {
        let neighbor = UVec2::new(cell.x, cell.y + 1);
        if structures.has_pipe_at(neighbor.x, neighbor.y) && !structures.is_pipe_cut(cell, neighbor)
        {
            mask |= 0b0100;
        }
    }
    if cell.x > 0 {
        let neighbor = UVec2::new(cell.x - 1, cell.y);
        if structures.has_pipe_at(neighbor.x, neighbor.y) && !structures.is_pipe_cut(cell, neighbor)
        {
            mask |= 0b1000;
        }
    }
    mask
}

fn vec4_to_color(value: Vec4) -> Color {
    Color::linear_rgba(value.x, value.y, value.z, value.w)
}

fn local_appearance_z(draw_priority: i32, draw_rank: usize) -> f32 {
    const APPEARANCE_PRIORITY_STEP: f32 = 0.000001;
    const APPEARANCE_TIEBREAK_STEP: f32 = 0.0000000001;
    let offset = draw_priority as f32 * APPEARANCE_PRIORITY_STEP
        + draw_rank as f32 * APPEARANCE_TIEBREAK_STEP;
    offset.clamp(-OVERLAY_LAYER_STEP_Z * 0.4, OVERLAY_LAYER_STEP_Z * 0.4)
}

#[cfg(test)]
mod tests {
    use super::{local_appearance_z, pipe_connection_mask, structure_transform};
    use crate::world::{
        grid::{cell_center, WorldGrid},
        structures::{PlacedStructureMap, StructureRotation},
    };
    use bevy::math::UVec2;

    #[test]
    fn overlay_local_subject_z_offset_stays_inside_layer_budget() {
        let offset = local_appearance_z(10_000, 100_000);
        assert!(offset <= 0.008);
        assert!(offset >= -0.008);
    }

    #[test]
    fn center_pipe_under_bridge_uses_neighbor_connections_for_mask() {
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        structures
            .place_bridge(UVec2::new(20, 20), StructureRotation::Deg0, &world)
            .expect("bridge should be placeable");
        assert!(structures.place_pipe(21, 20, &world));
        assert!(structures.place_pipe(20, 20, &world));
        assert!(structures.place_pipe(22, 20, &world));

        let center_mask = pipe_connection_mask(&structures, UVec2::new(21, 20));
        assert_eq!(center_mask, 0b1010, "center pipe must connect left/right");
    }

    #[test]
    fn pump_structure_transform_is_centered_between_two_cells() {
        let world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        let origin = UVec2::new(30, 30);
        let _ = structures
            .place_gas_pump(origin, StructureRotation::Deg0, &world)
            .expect("pump should be placeable");
        let pump = structures
            .iter()
            .find(|structure| crate::plugins::default_plugin::is_gas_pump_structure(structure.kind))
            .expect("pump structure");
        let transform = structure_transform(pump, 1.0);
        let expected =
            (cell_center(origin.x, origin.y) + cell_center(origin.x + 1, origin.y)) * 0.5;
        assert!((transform.translation.x - expected.x).abs() < 0.001);
        assert!((transform.translation.y - expected.y).abs() < 0.001);
    }
}
