```rust
impl MyPlugin {
    fn on_mouse_down(&mut self, event: &MouseCellEvent) -> Result<(), PluginError> {
        if event.button != Some(MouseButton::Right) || !self.world.is_editable(event.cell) {
            return Ok(());
        }
        self.dragging = true;
        self.last_cell = event.cell;
        self.entities.place(
            self.metal_id.clone(),
            EntityPlacement {
                origin: event.cell,
                rotation: Rotation::Deg0,
            },
        )?;
        Ok(())
    }
}
```
