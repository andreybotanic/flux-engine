```rust
impl MyPlugin {
    fn on_tool_selected(&mut self, event: &ToolSelectedEvent) -> Result<(), PluginError> {
        self.active_tool = event.tool_id.clone();
        Ok(())
    }
}
```
