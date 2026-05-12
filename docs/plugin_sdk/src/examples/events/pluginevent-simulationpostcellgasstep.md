```rust
impl MyPlugin {
    fn on_simulation_post_step(
        &mut self,
        _event: &SimulationPostCellGasStepEvent,
    ) -> Result<(), PluginError> {
        let pressure = self.gases.pressure_at(self.inject_cell)?;
        self.last_pressure = pressure;
        Ok(())
    }
}
```
