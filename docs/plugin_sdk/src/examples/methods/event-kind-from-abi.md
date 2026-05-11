```rust
let engine_kind = event_kind_from_abi(FluxEventKind::BuildPanel.as_raw())
    .expect("known ABI event tag");
assert_eq!(engine_kind, PluginEventKind::BuildPanel);
```
