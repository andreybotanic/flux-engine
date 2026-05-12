```rust
impl MyPlugin {
    fn on_entity_placed(&mut self, event: &EntityEvent) -> Result<(), PluginError> {
        self.log.info(format!(
            "entity '{}' placed at ({}, {})",
            event.kind, event.cell.x, event.cell.y
        ))?;
        Ok(())
    }
}
```
