```rust
unsafe fn paint_cell(
    callback: FluxSetCellMaterialFn,
    context: *mut std::ffi::c_void,
) -> FluxStatus {
    unsafe {
        callback(
            context,
            20,
            12,
            FluxUtf8Slice::from_str("flux.default.cell.metal"),
        )
    }
}
```
