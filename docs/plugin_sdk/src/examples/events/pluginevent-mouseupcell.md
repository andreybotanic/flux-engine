```rust
impl MyPlugin {
    fn on_mouse_up(&mut self, event: &MouseCellEvent) -> Result<(), PluginError> {
        self.dragging = false;
        self.last_cell = event.cell;
        Ok(())
    }
}
```
