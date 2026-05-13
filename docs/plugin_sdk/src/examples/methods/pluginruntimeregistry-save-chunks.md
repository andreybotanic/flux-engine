```rust
let save_chunks = registry.save_chunks();
assert!(save_chunks.values().all(|chunk| chunk.version >= 1));
```
