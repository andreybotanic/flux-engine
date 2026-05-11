```rust
let tool_id = ContentId::parse("flux.demo.tool.paint").expect("valid tool id");
let raw = tool_id.as_str();
assert!(raw.starts_with("flux.demo.tool."));
```
