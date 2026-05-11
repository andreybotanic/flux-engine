```rust
let overlay_id = ContentId::parse("flux.demo.overlay.temperature").expect("valid overlay id");
let mut frame = OverlayFrame::plugin_controlled(overlay_id);
frame.draw_commands.push(OverlayDrawCommand::Text {
    cell: UVec2::new(10, 10),
    text: "hot".to_string(),
    color: Color::srgb(1.0, 0.2, 0.2),
    z: 2.0,
});
assert_eq!(frame.policy, OverlayRenderPolicy::PluginControlled);
```
