```rust
impl MyPlugin {
    fn on_world_unloaded(&mut self, _event: &WorldUnloadedEvent) -> Result<(), PluginError> {
        self.dragging = false;
        self.cached_selection = None;
        Ok(())
    }
}
```
