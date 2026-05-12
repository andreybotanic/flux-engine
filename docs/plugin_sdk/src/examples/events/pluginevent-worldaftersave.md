```rust
impl MyPlugin {
    fn on_world_after_save(&mut self, _event: &WorldAfterSaveEvent) -> Result<(), PluginError> {
        self.log.info("plugin data was written into the save slot")?;
        Ok(())
    }
}
```
