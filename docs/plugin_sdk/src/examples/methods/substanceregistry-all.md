```rust
let definitions = registry.all();
let labels: Vec<_> = definitions.iter().map(|definition| definition.label.as_str()).collect();
assert!(!labels.is_empty());
```
