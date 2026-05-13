```rust
unsafe fn report_error(callback: FluxWriteErrorFn, context: *mut std::ffi::c_void) -> FluxStatus {
    let status = unsafe { callback(context, FluxUtf8Slice::from_str("plugin failed to initialize")) };
    if status.is_ok() { FluxStatus::OK } else { status }
}
```
