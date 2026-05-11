```rust
let bytes = br#"
id = "flux.demo"
display_name = "Flux Demo"
version = "1.2.3"
api_version = 4
dll = "bin/flux_demo.dll"
configs = "config"
assets = "assets"
content = false
"#;
let manifest = PluginManifest::from_bytes(bytes).expect("manifest bytes should parse");
assert_eq!(manifest.version.as_semver().major, 1);
```
