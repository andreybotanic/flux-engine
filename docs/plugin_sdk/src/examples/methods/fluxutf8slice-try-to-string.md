```rust
fn read_overlay_id(raw: FluxUtf8Slice) -> Result<String, FluxStatus> {
    let overlay_id = raw.try_to_string()?;
    println!("overlay id: {overlay_id}");
    Ok(overlay_id)
}
```
