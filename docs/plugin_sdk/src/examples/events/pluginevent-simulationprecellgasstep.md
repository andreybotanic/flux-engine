```rust
impl MyPlugin {
    fn on_simulation_pre_step(
        &mut self,
        _event: &SimulationPreCellGasStepEvent,
    ) -> Result<(), PluginError> {
        if self.time.is_paused()? {
            return Ok(());
        }
        self.gases.add(self.inject_cell, self.neon_id.clone(), 4)
            .map(|_| ())
    }
}
```
