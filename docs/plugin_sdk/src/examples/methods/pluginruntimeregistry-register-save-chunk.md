```rust
let mut registry = PluginRuntimeRegistry::default();
let descriptor = SaveChunkDescriptor {
    id: ContentId::parse("flux.demo.save.counter").expect("chunk id"),
    plugin_id: PluginId::parse("flux.demo").expect("plugin id"),
    version: 1,
};
registry.register_save_chunk(descriptor);
assert!(registry.save_chunks().contains_key(&ContentId::parse("flux.demo.save.counter").expect("chunk id")));
```
