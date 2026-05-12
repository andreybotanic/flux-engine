```rust
impl MyPlugin {
    fn on_render_overlay(&mut self, event: &RenderOverlayEvent) -> Result<(), PluginError> {
        let width = 102;
        let height = 102;
        let rgba8 = vec![0; (width * height * 4) as usize];
        self.overlays.submit_frame(OverlayFrame {
            overlay_id: event.overlay_id.clone(),
            width,
            height,
            rgba8,
        })
    }
}
```
