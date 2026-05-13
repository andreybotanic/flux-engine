```rust
let name = canonical_handler_name(PluginEvent::MouseDownCell);
let descriptor = FluxEventHandlerDescriptor::new(
    FluxEventKind::MouseDownCell,
    FluxUtf8Slice::from_str(name),
);
assert_eq!(descriptor.event_kind, FluxEventKind::MouseDownCell.as_raw());
```
