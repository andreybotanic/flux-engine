fn cell_descriptor_for(
    id: &str,
    material: CellMaterial,
    config_file_name: &'static str,
    label: &str,
    image_path: &str,
    silhouette_path: Option<&str>,
    storage_code: u8,
) -> CellContentDescriptor {
    CellContentDescriptor {
        id: content_id(id),
        plugin_id: PluginId::default_plugin(),
        material,
        config_file_name,
        visual: VisualPlacementConfig {
            label: label.to_string(),
            draw_priority: 1000,
            size_in_cells: UVec2::ONE,
        },
        layer_descriptor: cell_layers(material),
        sprite: sprite(image_path, silhouette_path, None),
        storage: LegacyStorageDescriptor::WorldCellCode(storage_code),
    }
}

fn structure_descriptor_for(
    id: &str,
    kind: StructureKind,
    config_file_name: &'static str,
    label: &str,
    draw_priority: i32,
    size_in_cells: UVec2,
    image_path: &str,
    silhouette_path: Option<&str>,
    overlay_path: Option<&str>,
    allowed_rotations: Vec<StructureRotation>,
    hud: HudBlockConfig,
    storage_kind: &'static str,
) -> StructureContentDescriptor {
    StructureContentDescriptor {
        id: content_id(id),
        plugin_id: PluginId::default_plugin(),
        kind,
        config_file_name,
        visual: VisualPlacementConfig {
            label: label.to_string(),
            draw_priority,
            size_in_cells,
        },
        layer_descriptors: structure_layers_by_rotation(kind),
        allowed_rotations,
        sprite: sprite(image_path, silhouette_path, overlay_path),
        hud,
        storage: LegacyStorageDescriptor::PlacedStructureKind(storage_kind),
    }
}

fn overlay_descriptor(
    id: &str,
    mode: OverlayMode,
    label: &'static str,
    hotkey: &'static str,
) -> OverlayContentDescriptor {
    OverlayContentDescriptor {
        id: content_id(id),
        plugin_id: PluginId::default_plugin(),
        mode,
        label,
        hotkey,
        storage: LegacyStorageDescriptor::OverlayMode(label),
    }
}

fn content_id(raw: &str) -> ContentId {
    ContentId::parse(raw).expect("default plugin content id must stay valid")
}

fn default_gas_substance(
    raw_id: &str,
    alias: &str,
    label: &str,
    molecular_mass: f32,
    color: [f32; 3],
) -> SubstanceDefinition {
    SubstanceDefinition::gas(
        SubstanceId::parse(raw_id).expect("default plugin substance id must stay valid"),
        PluginId::default_plugin(),
        label,
        molecular_mass,
        color,
        vec![alias.to_string()],
    )
    .expect("default plugin substance definition must stay valid")
}

fn sprite(
    image_path: &str,
    silhouette_path: Option<&str>,
    overlay_path: Option<&str>,
) -> SpriteMetadata {
    SpriteMetadata {
        image_path: image_path.to_string(),
        silhouette_path: silhouette_path.map(ToString::to_string),
        overlay_path: overlay_path.map(ToString::to_string),
    }
}

fn title_only_hud_block(sort_order: i32) -> HudBlockConfig {
    HudBlockConfig {
        sort_order,
        substance_containers: Vec::new(),
    }
}

fn pipe_hud_block() -> HudBlockConfig {
    HudBlockConfig {
        sort_order: 10,
        substance_containers: vec![SubstanceContainerConfig {
            substance: SubstanceKind::Gas,
            backing: ContainerBacking::PipeNode {
                kind: ConfiguredPipeNodeKind::Pipe,
            },
            visible_on_hover: HoverVisibility::SameCell,
        }],
    }
}

fn bridge_hud_block() -> HudBlockConfig {
    HudBlockConfig {
        sort_order: 20,
        substance_containers: vec![SubstanceContainerConfig {
            substance: SubstanceKind::Gas,
            backing: ContainerBacking::PipeNode {
                kind: ConfiguredPipeNodeKind::BridgePipe,
            },
            visible_on_hover: HoverVisibility::ContainerCell,
        }],
    }
}

fn cell_layers(material: CellMaterial) -> StructureDescriptor {
    StructureDescriptor {
        layers: vec![StructureLayer {
            kind: LayerKind::Appearance,
            cells: vec![LayerCellSpec {
                local_cell: IVec2::ZERO,
                marker_kind: LayerMarkerKind::Solid(material),
                collision: LayerCollisionKind::Special,
            }],
        }],
    }
}

fn structure_layers_by_rotation(
    kind: StructureKind,
) -> BTreeMap<StructureRotation, StructureDescriptor> {
    [
        StructureRotation::Deg0,
        StructureRotation::Deg90,
        StructureRotation::Deg180,
        StructureRotation::Deg270,
    ]
    .into_iter()
    .map(|rotation| (rotation, structure_layers(kind, rotation)))
    .collect()
}

fn structure_layers(kind: StructureKind, rotation: StructureRotation) -> StructureDescriptor {
    match kind {
        StructureKind::Pipe => StructureDescriptor {
            layers: vec![StructureLayer {
                kind: LayerKind::Appearance,
                cells: vec![LayerCellSpec {
                    local_cell: IVec2::ZERO,
                    marker_kind: LayerMarkerKind::Pipe,
                    collision: LayerCollisionKind::RenderOnly,
                }],
            }],
        },
        StructureKind::Vent => StructureDescriptor {
            layers: vec![
                StructureLayer {
                    kind: LayerKind::Appearance,
                    cells: vec![LayerCellSpec {
                        local_cell: IVec2::ZERO,
                        marker_kind: LayerMarkerKind::Vent,
                        collision: LayerCollisionKind::Special,
                    }],
                },
                StructureLayer {
                    kind: LayerKind::GasPipeConnections,
                    cells: vec![LayerCellSpec {
                        local_cell: IVec2::ZERO,
                        marker_kind: LayerMarkerKind::GasPipeConnectionBidirectional,
                        collision: LayerCollisionKind::Special,
                    }],
                },
            ],
        },
        StructureKind::GasSource => one_cell_structure(LayerMarkerKind::GasSource),
        StructureKind::GasSink => one_cell_structure(LayerMarkerKind::GasSink),
        StructureKind::GasPipeBridge => bridge_layers(rotation),
    }
}

fn one_cell_structure(marker_kind: LayerMarkerKind) -> StructureDescriptor {
    StructureDescriptor {
        layers: vec![StructureLayer {
            kind: LayerKind::Appearance,
            cells: vec![LayerCellSpec {
                local_cell: IVec2::ZERO,
                marker_kind,
                collision: LayerCollisionKind::Special,
            }],
        }],
    }
}

fn bridge_layers(rotation: StructureRotation) -> StructureDescriptor {
    StructureDescriptor {
        layers: vec![
            StructureLayer {
                kind: LayerKind::Appearance,
                cells: bridge_local_cells(rotation)
                    .into_iter()
                    .map(|local_cell| LayerCellSpec {
                        local_cell,
                        marker_kind: LayerMarkerKind::GasPipeBridge,
                        collision: LayerCollisionKind::RenderOnly,
                    })
                    .collect(),
            },
            StructureLayer {
                kind: LayerKind::GasPipeConnections,
                cells: bridge_connection_local_cells(rotation)
                    .into_iter()
                    .map(|local_cell| LayerCellSpec {
                        local_cell,
                        marker_kind: LayerMarkerKind::GasPipeConnectionBidirectional,
                        collision: LayerCollisionKind::Special,
                    })
                    .collect(),
            },
        ],
    }
}

fn bridge_local_cells(rotation: StructureRotation) -> Vec<IVec2> {
    match rotation {
        StructureRotation::Deg0 | StructureRotation::Deg180 => {
            vec![IVec2::new(0, 0), IVec2::new(1, 0), IVec2::new(2, 0)]
        }
        StructureRotation::Deg90 | StructureRotation::Deg270 => {
            vec![IVec2::new(0, 0), IVec2::new(0, 1), IVec2::new(0, 2)]
        }
    }
}

fn bridge_connection_local_cells(rotation: StructureRotation) -> Vec<IVec2> {
    match rotation {
        StructureRotation::Deg0 | StructureRotation::Deg180 => {
            vec![IVec2::new(0, 0), IVec2::new(2, 0)]
        }
        StructureRotation::Deg90 | StructureRotation::Deg270 => {
            vec![IVec2::new(0, 0), IVec2::new(0, 2)]
        }
    }
}
