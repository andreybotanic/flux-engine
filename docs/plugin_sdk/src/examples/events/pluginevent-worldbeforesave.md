```rust
impl MyPlugin {
    fn on_world_before_save(
        &mut self,
        _event: &WorldBeforeSaveEvent,
    ) -> Result<(), PluginError> {
        self.save
            .write_json(self.settings_chunk_id.clone(), 1, &self.settings)
    }
}
```
