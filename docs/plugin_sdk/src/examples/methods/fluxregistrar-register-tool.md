```rust
fn register_tool(registrar: &mut FluxRegistrar) -> FluxStatus {
    let descriptor = FluxToolDescriptor {
        id: FluxUtf8Slice::from_str("flux.demo.tool.paint"),
        label: FluxUtf8Slice::from_str("Demo Paint"),
        icon_path: FluxUtf8Slice::from_str("assets/paint.png"),
        silhouette_path: FluxUtf8Slice::from_str("assets/paint_silhouette.png"),
    };
    match registrar.register_tool(&descriptor) {
        Ok(()) => FluxStatus::OK,
        Err(status) => status,
    }
}
```
