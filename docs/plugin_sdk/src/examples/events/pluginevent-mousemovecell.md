```rust
impl MyPlugin {
    fn on_mouse_move(&mut self, event: &MouseCellEvent) -> Result<(), PluginError> {
        if !self.dragging || !self.world.contains(event.cell) {
            return Ok(());
        }
        for cell in self.world.ray_cells(self.last_cell, event.cell) {
            if self.world.is_editable(cell) {
                self.entities.place(
                    self.metal_id.clone(),
                    EntityPlacement {
                        origin: cell,
                        rotation: Rotation::Deg0,
                    },
                )?;
            }
        }
        self.last_cell = event.cell;
        Ok(())
    }
}
```
