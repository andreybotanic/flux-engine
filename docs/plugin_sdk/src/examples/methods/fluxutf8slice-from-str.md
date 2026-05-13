```rust
let slice = FluxUtf8Slice::from_str("flux.demo.overlay.heat");
let bytes = unsafe { std::slice::from_raw_parts(slice.ptr, slice.len) };
assert_eq!(std::str::from_utf8(bytes).unwrap(), "flux.demo.overlay.heat");
```
