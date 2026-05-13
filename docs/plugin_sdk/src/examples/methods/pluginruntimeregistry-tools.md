```rust
let tools = registry.tools();
assert!(tools.values().any(|tool| tool.label.contains("Paint")));
```
