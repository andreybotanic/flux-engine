```rust
let descriptor = StructureDescriptor {
    layers: vec![
        StructureLayer {
            kind: LayerKind::new("flux.core.layer.appearance"),
            cells: vec![
                LayerCellSpec {
                    local_cell: IVec2::ZERO,
                    marker_kind: LayerMarkerKind::new("flux.demo.marker.core"),
                    collision: LayerCollisionKind::Special,
                },
                LayerCellSpec {
                    local_cell: IVec2::new(1, 0),
                    marker_kind: LayerMarkerKind::new("flux.demo.marker.core"),
                    collision: LayerCollisionKind::Special,
                },
            ],
        },
        StructureLayer {
            kind: LayerKind::new("flux.demo.layer.overlay"),
            cells: vec![LayerCellSpec {
                local_cell: IVec2::new(1, 0),
                marker_kind: LayerMarkerKind::new("flux.demo.marker.overlay"),
                collision: LayerCollisionKind::RenderOnly,
            }],
        },
    ],
};
let occupied = descriptor.occupied_local_cells();
assert_eq!(occupied.len(), 2);
```
