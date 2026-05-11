```rust
fn paint_metal(host: &mut FluxRuntimeHost, cell: UVec2) -> FluxStatus {
    match host.set_cell_material(cell, "flux.default.cell.metal") {
        Ok(()) => FluxStatus::OK,
        Err(status) => status,
    }
}
```
