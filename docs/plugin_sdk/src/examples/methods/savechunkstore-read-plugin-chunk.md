```rust
let chunk = store
    .read_plugin_chunk(&plugin_id, &chunk_id)
    .expect("plugin should have saved this chunk");
assert_eq!(chunk.bytes.len(), 4);
```
