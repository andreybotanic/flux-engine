```rust
impl MyPlugin {
    fn on_entity_removed(&mut self, event: &EntityEvent) -> Result<(), PluginError> {
        if self.selected_entity == Some(event.id) {
            self.selected_entity = None;
        }
        Ok(())
    }
}
```
