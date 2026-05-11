```rust
let manifest = PluginManifest::from_str(r#"
id = "flux.demo"
display_name = "Flux Demo"
version = "1.2.3"
api_version = 4
dll = "bin/flux_demo.dll"
configs = "config"
assets = "assets"
content = true
"#).expect("manifest should parse");
assert_eq!(manifest.id.as_str(), "flux.demo");
```
