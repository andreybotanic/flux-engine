```rust
let chunks = store.chunks();
let total_bytes: usize = chunks.iter().map(|chunk| chunk.bytes.len()).sum();
assert!(total_bytes >= chunks.len());
```
