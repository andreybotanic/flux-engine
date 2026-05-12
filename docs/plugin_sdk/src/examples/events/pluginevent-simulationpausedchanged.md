```rust
impl MyPlugin {
    fn on_simulation_paused_changed(
        &mut self,
        event: &SimulationPausedChangedEvent,
    ) -> Result<(), PluginError> {
        self.log.info(format!("simulation paused: {}", event.paused))?;
        Ok(())
    }
}
```
