```rust
let version = PluginVersion::parse("2.0.1").expect("valid semver");
let semver = version.as_semver();
assert_eq!((semver.major, semver.minor, semver.patch), (2, 0, 1));
```
