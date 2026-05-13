```rust
fn vent_hydrogen(host: &mut FluxRuntimeHost, cell: UVec2) -> FluxStatus {
    let velocity = Vec2::new(2.0, 0.0);
    let added = match host.add_gas(cell, "h2", 25, velocity) {
        Ok(added) => added,
        Err(status) => return status,
    };
    println!("added {added} hydrogen particles");
    FluxStatus::OK
}
```
