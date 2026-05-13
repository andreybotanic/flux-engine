```rust
fn register_oxygen(registrar: &mut FluxRegistrar) -> FluxStatus {
    let descriptor = FluxGasSubstanceDescriptor {
        id: FluxUtf8Slice::from_str("flux.demo.gas.oxygen"),
        label: FluxUtf8Slice::from_str("Demo Oxygen"),
        alias: FluxUtf8Slice::from_str("demo_o2"),
        molecular_mass: 32.0,
        color_r: 0.55,
        color_g: 0.8,
        color_b: 1.0,
    };
    match registrar.register_gas_substance(&descriptor) {
        Ok(()) => FluxStatus::OK,
        Err(status) => status,
    }
}
```
