```rust
let mut registry = PluginRuntimeRegistry::default();
let descriptor = ToolDescriptor {
    id: ContentId::parse("flux.demo.tool.paint").expect("tool id"),
    label: "Paint".to_string(),
    icon_path: "icons/paint.png".to_string(),
    silhouette_path: Some("icons/paint_silhouette.png".to_string()),
};
registry.register_tool(descriptor);
assert!(registry.tools().contains_key(&ContentId::parse("flux.demo.tool.paint").expect("tool id")));
```
