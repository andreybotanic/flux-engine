```rust
unsafe fn register_oxygen(
    callback: FluxRegisterGasSubstanceFn,
    context: *mut std::ffi::c_void,
) -> FluxStatus {
    let descriptor = FluxGasSubstanceDescriptor {
        id: FluxUtf8Slice::from_str("flux.demo.gas.oxygen"),
        label: FluxUtf8Slice::from_str("Oxygen"),
        alias: FluxUtf8Slice::from_str("o2"),
        molecular_mass: 32.0,
        color_r: 0.4,
        color_g: 0.7,
        color_b: 1.0,
    };
    unsafe { callback(context, &descriptor) }
}
```
