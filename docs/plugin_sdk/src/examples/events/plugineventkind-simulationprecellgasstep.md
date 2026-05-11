```rust
unsafe extern "C" fn on_simulation_pre_cell_gas_step(
    _plugin: *mut FluxPluginHandle,
    payload: *const FluxEmptyEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let _payload = unsafe { &*payload };
    let host = unsafe { &mut *host };
    let Some(add_gas) = host.add_gas else {
        return FluxStatus::FAILED;
    };

    let mut added = 0u32;
    let status = unsafe {
        add_gas(
            host.context,
            20,
            20,
            FluxUtf8Slice::from_str("flux.default.gas.oxygen"),
            25,
            0.0,
            1.0,
            &mut added,
        )
    };
    if !status.is_ok() || added == 0 {
        return FluxStatus::FAILED;
    }
    FluxStatus::OK
}
```
