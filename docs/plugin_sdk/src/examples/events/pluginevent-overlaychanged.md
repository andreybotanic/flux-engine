```rust
impl MyPlugin {
    fn on_overlay_changed(&mut self, event: &OverlayChangedEvent) -> Result<(), PluginError> {
        self.last_overlay = event.overlay_id.clone();
        Ok(())
    }
}
```
