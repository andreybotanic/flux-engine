```rust
impl MyPlugin {
    fn on_world_loaded(&mut self, _event: &WorldLoadedEvent) -> Result<(), PluginError> {
        if let Some(saved) = self.save.read_json::<PluginSettings>(&self.settings_chunk_id)? {
            self.settings = saved;
        }
        self.log.info("plugin state restored after world load")
    }
}
```
