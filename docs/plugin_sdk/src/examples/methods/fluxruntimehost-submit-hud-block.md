```rust
fn send_hud_line(host: &mut FluxRuntimeHost, total_clicks: u32) -> FluxStatus {
    let line = format!("total clicks {total_clicks}");
    match host.submit_hud_block("Demo Stats", &line) {
        Ok(()) => FluxStatus::OK,
        Err(status) => status,
    }
}
```
