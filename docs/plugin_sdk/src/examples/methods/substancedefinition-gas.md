```rust
let definition = SubstanceDefinition::gas(
    SubstanceId::parse("flux.demo.gas.oxygen").expect("id"),
    PluginId::parse("flux.demo").expect("plugin id"),
    "Oxygen",
    32.0,
    [0.3, 0.7, 1.0],
    vec!["o2".to_string()],
).expect("definition should validate");
assert!(definition.flags.gas);
```
