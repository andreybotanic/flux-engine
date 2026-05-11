```rust
let registry = SubstanceRegistry::new(vec![
    SubstanceDefinition::gas(
        SubstanceId::parse("flux.demo.gas.hydrogen").expect("id"),
        PluginId::parse("flux.demo").expect("plugin id"),
        "Hydrogen",
        2.0,
        [0.8, 0.9, 1.0],
        vec!["h2".to_string()],
    ).expect("definition"),
    SubstanceDefinition::gas(
        SubstanceId::parse("flux.demo.gas.oxygen").expect("id"),
        PluginId::parse("flux.demo").expect("plugin id"),
        "Oxygen",
        32.0,
        [0.3, 0.7, 1.0],
        vec!["o2".to_string()],
    ).expect("definition"),
]).expect("registry should validate");
assert_eq!(registry.count(), 2);
```
