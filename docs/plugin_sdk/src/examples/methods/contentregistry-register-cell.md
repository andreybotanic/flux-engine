```rust
let mut registry = ContentRegistry::default();
let descriptor = CellContentDescriptor {
    id: ContentId::parse("flux.demo.cell.steel").expect("id"),
    plugin_id: PluginId::parse("flux.demo").expect("plugin id"),
    material: CellMaterial::new("flux.demo.cell.steel"),
    config_file_name: "steel.toml",
    visual: VisualPlacementConfig {
        label: "Steel".to_string(),
        draw_priority: 10,
        size_in_cells: UVec2::ONE,
    },
    layer_descriptor: StructureDescriptor {
        layers: vec![StructureLayer {
            kind: LayerKind::new("flux.core.layer.appearance"),
            cells: vec![LayerCellSpec {
                local_cell: IVec2::ZERO,
                marker_kind: LayerMarkerKind::new("flux.demo.marker.wall"),
                collision: LayerCollisionKind::Special,
            }],
        }],
    },
    sprite: SpriteMetadata {
        image_path: "cells/steel.png".to_string(),
        silhouette_path: Some("cells/steel_silhouette.png".to_string()),
        overlay_path: None,
    },
    storage: LegacyStorageDescriptor::WorldCellCode(42),
};
registry.register_cell(descriptor);
assert!(registry.cell_by_material(CellMaterial::new("flux.demo.cell.steel")).is_some());
```
