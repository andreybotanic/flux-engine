```rust
let manifest = PluginManifest::from_str(manifest_text)?;
if manifest.api_version != ENGINE_PLUGIN_API_VERSION {
    return Err(format!(
        "plugin expects API {}, engine provides {}",
        manifest.api_version,
        ENGINE_PLUGIN_API_VERSION,
    ));
}
```
