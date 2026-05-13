```rust
impl MyPlugin {
    fn on_world_created(&mut self, _event: &WorldCreatedEvent) -> Result<(), PluginError> {
        self.log.info("plugin observed creation of a new world")?;
        self.time.set_paused(true)
    }
}
```
