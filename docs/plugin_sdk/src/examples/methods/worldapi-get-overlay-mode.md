```rust
if let Some(mode) = world.get_overlay_mode() {
    if mode.is_gas() {
        println!("gas overlay is active");
    }
}
```
