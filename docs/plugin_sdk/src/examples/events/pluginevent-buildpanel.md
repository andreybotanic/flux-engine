```rust
impl MyPlugin {
    fn on_build_panel(&mut self, event: &BuildPanelEvent) -> Result<(), PluginError> {
        if event.panel_id != self.panel_id {
            return Ok(());
        }
        self.panels.set_root(
            &self.panel_id,
            UiNode::Text {
                text: format!("selected cell: {:?}", self.hovered_cell),
            },
        )
    }
}
```
