```rust
let descriptor = FluxEventHandlerDescriptor::new(
    FluxEventKind::BuildHudForCell,
    FluxUtf8Slice::from_str("onBuildHudForCell"),
);
assert_eq!(descriptor.event_kind, FluxEventKind::BuildHudForCell.as_raw());
```
