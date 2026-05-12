```rust
impl MyPlugin {
    fn on_key_released(&mut self, event: &KeyEvent) -> Result<(), PluginError> {
        self.log.debug(format!("key released: {}", event.key))?;
        Ok(())
    }
}
```
