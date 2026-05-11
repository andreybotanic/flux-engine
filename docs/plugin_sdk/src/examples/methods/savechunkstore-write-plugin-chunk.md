```rust
let plugin_id = PluginId::parse("flux.demo").expect("plugin id");
let chunk_id = ContentId::parse("flux.demo.save.counter").expect("chunk id");
let mut store = SaveChunkStore::default();
store.write_plugin_chunk(plugin_id.clone(), chunk_id.clone(), 1, 7u32.to_le_bytes().to_vec());
let chunk = store.read_plugin_chunk(&plugin_id, &chunk_id).expect("chunk should exist");
assert_eq!(chunk.version, 1);
```
