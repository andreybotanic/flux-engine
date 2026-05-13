```rust
impl MyPlugin {
    fn on_key_pressed(&mut self, event: &KeyEvent) -> Result<(), PluginError> {
        if event.key == "Space" {
            let _ = self.time.toggle_pause()?;
        }
        Ok(())
    }
}
```
