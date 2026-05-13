```rust
impl MyPlugin {
    fn on_render_overlay(&mut self, event: &RenderOverlayEvent) -> Result<(), PluginError> {
        if event.overlay_id.as_str() != "flux.demo.overlay.pipes" {
            return Ok(());
        }
        let root = OverlayNodeId::parse("pipes.root").expect("valid node id");
        self.overlays.submit_graph(OverlayGraph {
            nodes: vec![OverlayNode {
                id: root.clone(),
                depends_on: vec![],
                kind: OverlayNodeKind::RenderImage(RenderImageNode {
                    instances: Vec::new(),
                }),
            }],
            output: root,
        })
    }
}
```
