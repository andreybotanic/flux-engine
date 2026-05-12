```rust
impl MyPlugin {
    fn on_mouse_leave(&mut self, _event: &MouseCellEvent) -> Result<(), PluginError> {
        self.hovered_cell = None;
        Ok(())
    }
}
```
