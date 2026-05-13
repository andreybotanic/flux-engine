```rust
impl MyPlugin {
    fn on_mouse_enter(&mut self, event: &MouseCellEvent) -> Result<(), PluginError> {
        self.hovered_cell = Some(event.cell);
        Ok(())
    }
}
```
