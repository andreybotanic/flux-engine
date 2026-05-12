```rust
impl MyPlugin {
    fn on_build_hud_for_cell(
        &mut self,
        event: &BuildHudForCellEvent,
    ) -> Result<(), PluginError> {
        let pressure = self.gases.pressure_at(event.cell)?;
        self.ui.add_hud_line(
            self.hud_block_id.clone(),
            "Pressure probe".to_string(),
            format!("cell ({}, {}): {:.2} Pa", event.cell.x, event.cell.y, pressure),
        )
    }
}
```
