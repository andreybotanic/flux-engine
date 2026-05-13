```rust
let version = PluginVersion::parse("1.4.0-beta.2").expect("valid semver");
assert_eq!(version.as_semver().minor, 4);
```
