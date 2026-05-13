```rust
let mut registry = ContentRegistry::default();
registry.set_world_cell_hud(WorldCellHudConfig {
    label: "Hovered Cell".to_string(),
    block: HudBlockConfig {
        sort_order: 0,
        substance_containers: vec![SubstanceContainerConfig {
            substance: SubstanceKind::Gas,
            backing: ContainerBacking::WorldCell,
            visible_on_hover: HoverVisibility::SameCell,
        }],
    },
});
assert!(registry.world_cell_hud().is_some());
```
