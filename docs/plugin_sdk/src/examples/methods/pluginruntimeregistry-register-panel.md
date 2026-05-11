```rust
let mut registry = PluginRuntimeRegistry::default();
let descriptor = PanelDescriptor {
    id: ContentId::parse("flux.demo.panel.stats").expect("panel id"),
    title: "Stats".to_string(),
    root: UiNode::Column {
        children: vec![UiNode::Text {
            text: "Plugin statistics".to_string(),
        }],
    },
};
registry.register_panel(descriptor);
assert!(registry.panels().contains_key(&ContentId::parse("flux.demo.panel.stats").expect("panel id")));
```
