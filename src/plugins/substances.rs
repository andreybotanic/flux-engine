use std::{
    collections::{BTreeMap, HashMap},
    fmt,
};

use crate::plugins::PluginId;

/// Canonical identifier of one substance registered by a content plugin.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubstanceId(String);

impl SubstanceId {
    /// Parses and validates one substance identifier.
    pub fn parse(raw: &str) -> Result<Self, String> {
        validate_substance_id(raw)?;
        Ok(Self(raw.to_string()))
    }

    /// Returns the canonical string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the final id segment, useful for legacy short labels.
    pub fn leaf(&self) -> &str {
        self.0.rsplit('.').next().unwrap_or(self.0.as_str())
    }
}

impl fmt::Display for SubstanceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Feature flags declared for a plugin-owned substance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SubstanceFlags {
    pub gas: bool,
}

impl SubstanceFlags {
    /// Returns flags for a substance that can participate in gas simulation.
    pub fn gas() -> Self {
        Self { gas: true }
    }
}

/// Describes one plugin-owned substance before it is assigned a compact runtime index.
#[derive(Clone, Debug, PartialEq)]
pub struct SubstanceDefinition {
    pub id: SubstanceId,
    pub plugin_id: PluginId,
    pub label: String,
    pub molecular_mass: f32,
    pub color: [f32; 3],
    pub aliases: Vec<String>,
    pub flags: SubstanceFlags,
}

impl SubstanceDefinition {
    /// Builds a gas-capable substance definition and validates its data.
    pub fn gas(
        id: SubstanceId,
        plugin_id: PluginId,
        label: impl Into<String>,
        molecular_mass: f32,
        color: [f32; 3],
        aliases: Vec<String>,
    ) -> Result<Self, String> {
        let definition = Self {
            id,
            plugin_id,
            label: label.into(),
            molecular_mass,
            color,
            aliases,
            flags: SubstanceFlags::gas(),
        };
        validate_definition(&definition)?;
        Ok(definition)
    }
}

/// Registry of plugin-owned substances with deterministic compact runtime order.
#[derive(Clone, Debug, PartialEq)]
pub struct SubstanceRegistry {
    definitions: Vec<SubstanceDefinition>,
    by_id: BTreeMap<SubstanceId, usize>,
    by_lookup: HashMap<String, usize>,
}

impl SubstanceRegistry {
    /// Builds a registry and assigns compact indices by molecular mass and stable id.
    pub fn new(mut definitions: Vec<SubstanceDefinition>) -> Result<Self, String> {
        if definitions.is_empty() {
            return Err(
                "Substance registry is empty. Register at least one substance.".to_string(),
            );
        }

        for definition in &definitions {
            validate_definition(definition)?;
            validate_plugin_namespace(definition)?;
        }

        definitions.sort_by(|a, b| {
            a.molecular_mass
                .total_cmp(&b.molecular_mass)
                .then_with(|| a.id.cmp(&b.id))
        });

        let mut by_id = BTreeMap::new();
        let mut by_lookup = HashMap::new();
        for (idx, definition) in definitions.iter().enumerate() {
            if by_id.insert(definition.id.clone(), idx).is_some() {
                return Err(format!("Duplicate substance id '{}'", definition.id));
            }
            insert_lookup(&mut by_lookup, definition.id.as_str(), idx)?;
            for alias in &definition.aliases {
                insert_lookup(&mut by_lookup, alias, idx)?;
            }
        }

        Ok(Self {
            definitions,
            by_id,
            by_lookup,
        })
    }

    /// Returns all definitions in compact runtime order.
    pub fn all(&self) -> &[SubstanceDefinition] {
        &self.definitions
    }

    /// Returns the number of registered substances.
    pub fn count(&self) -> usize {
        self.definitions.len()
    }

    /// Returns a definition by compact runtime index.
    pub fn get(&self, index: usize) -> Option<&SubstanceDefinition> {
        self.definitions.get(index)
    }

    /// Returns a definition by stable substance id.
    pub fn get_by_id(&self, id: &SubstanceId) -> Option<&SubstanceDefinition> {
        self.by_id
            .get(id)
            .and_then(|index| self.definitions.get(*index))
    }

    /// Returns compact runtime index by stable id or registered alias.
    pub fn compact_index(&self, id_or_alias: &str) -> Option<usize> {
        self.by_lookup.get(id_or_alias).copied()
    }

    /// Returns a stable id by compact runtime index.
    pub fn stable_id_by_index(&self, index: usize) -> Option<&SubstanceId> {
        self.definitions.get(index).map(|definition| &definition.id)
    }

    /// Returns molecular masses in compact runtime order.
    pub fn molecular_masses(&self) -> Vec<f32> {
        self.definitions
            .iter()
            .map(|definition| definition.molecular_mass)
            .collect()
    }
}

fn insert_lookup(
    by_lookup: &mut HashMap<String, usize>,
    key: &str,
    index: usize,
) -> Result<(), String> {
    if key.trim().is_empty() {
        return Err("Substance alias must not be empty".to_string());
    }
    if by_lookup.insert(key.to_string(), index).is_some() {
        return Err(format!("Duplicate substance lookup id '{}'", key));
    }
    Ok(())
}

fn validate_definition(definition: &SubstanceDefinition) -> Result<(), String> {
    if definition.label.trim().is_empty() {
        return Err(format!("Substance '{}' has empty label", definition.id));
    }
    if !definition.molecular_mass.is_finite() || definition.molecular_mass <= 0.0 {
        return Err(format!(
            "Substance '{}' has invalid molecular_mass {}",
            definition.id, definition.molecular_mass
        ));
    }
    for component in definition.color {
        if !component.is_finite() || !(0.0..=1.0).contains(&component) {
            return Err(format!(
                "Substance '{}' has invalid color component {}",
                definition.id, component
            ));
        }
    }
    Ok(())
}

fn validate_plugin_namespace(definition: &SubstanceDefinition) -> Result<(), String> {
    let expected_prefix = format!("{}.", definition.plugin_id.as_str());
    if definition.id.as_str().starts_with(&expected_prefix) {
        return Ok(());
    }
    Err(format!(
        "Substance id '{}' is outside plugin namespace '{}'",
        definition.id, definition.plugin_id
    ))
}

fn validate_substance_id(raw: &str) -> Result<(), String> {
    if raw.is_empty() {
        return Err("substance id must not be empty".to_string());
    }

    let bytes = raw.as_bytes();
    let first = bytes[0];
    if !is_lower_ascii_alphanumeric(first) {
        return Err(format!(
            "substance id '{}' must start with a lowercase ASCII letter or digit",
            raw
        ));
    }

    let last = *bytes.last().expect("non-empty checked above");
    if !is_lower_ascii_alphanumeric(last) {
        return Err(format!(
            "substance id '{}' must end with a lowercase ASCII letter or digit",
            raw
        ));
    }

    let mut previous_was_separator = false;
    for byte in bytes {
        if is_lower_ascii_alphanumeric(*byte) {
            previous_was_separator = false;
            continue;
        }

        if matches!(*byte, b'.' | b'_' | b'-') && !previous_was_separator {
            previous_was_separator = true;
            continue;
        }

        return Err(format!(
            "substance id '{}' must match ^[a-z0-9]+([._-][a-z0-9]+)*$",
            raw
        ));
    }

    Ok(())
}

fn is_lower_ascii_alphanumeric(value: u8) -> bool {
    value.is_ascii_lowercase() || value.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::{SubstanceDefinition, SubstanceId, SubstanceRegistry};
    use crate::plugins::PluginId;

    #[test]
    fn substance_id_rejects_invalid_values() {
        assert!(SubstanceId::parse("flux.default.substance.h2").is_ok());
        assert!(SubstanceId::parse("Flux.default.substance.h2").is_err());
        assert!(SubstanceId::parse("flux..h2").is_err());
        assert!(SubstanceId::parse("flux.default.").is_err());
    }

    #[test]
    fn substance_registry_rejects_duplicate_ids_and_aliases() {
        let plugin_id = PluginId::default_plugin();
        let first = SubstanceDefinition::gas(
            SubstanceId::parse("flux.default.substance.h2").expect("id"),
            plugin_id.clone(),
            "Hydrogen",
            2.016,
            [0.8, 0.2, 0.9],
            vec!["h2".to_string()],
        )
        .expect("first");
        let second = SubstanceDefinition::gas(
            SubstanceId::parse("flux.default.substance.hydrogen").expect("id"),
            plugin_id,
            "Hydrogen duplicate alias",
            2.016,
            [0.8, 0.2, 0.9],
            vec!["h2".to_string()],
        )
        .expect("second");

        let err = SubstanceRegistry::new(vec![first, second]).expect_err("duplicate alias");
        assert!(err.contains("Duplicate substance lookup id"));
    }

    #[test]
    fn substance_registry_rejects_invalid_data_and_foreign_namespace() {
        let plugin_id = PluginId::default_plugin();
        assert!(SubstanceDefinition::gas(
            SubstanceId::parse("flux.default.substance.bad").expect("id"),
            plugin_id.clone(),
            "Bad",
            0.0,
            [0.0, 0.0, 0.0],
            vec!["bad".to_string()],
        )
        .is_err());

        let foreign = SubstanceDefinition::gas(
            SubstanceId::parse("other.plugin.substance.h2").expect("id"),
            plugin_id,
            "Foreign",
            1.0,
            [0.0, 0.0, 0.0],
            vec!["foreign".to_string()],
        )
        .expect("definition");
        let err = SubstanceRegistry::new(vec![foreign]).expect_err("foreign namespace");
        assert!(err.contains("outside plugin namespace"));
    }

    #[test]
    fn substance_registry_orders_by_mass_then_stable_id() {
        let plugin_id = PluginId::default_plugin();
        let registry = SubstanceRegistry::new(vec![
            SubstanceDefinition::gas(
                SubstanceId::parse("flux.default.substance.co2").expect("id"),
                plugin_id.clone(),
                "CO2",
                44.009,
                [0.5, 0.5, 0.5],
                vec!["co2".to_string()],
            )
            .expect("co2"),
            SubstanceDefinition::gas(
                SubstanceId::parse("flux.default.substance.h2").expect("id"),
                plugin_id.clone(),
                "H2",
                2.016,
                [0.8, 0.2, 0.9],
                vec!["h2".to_string()],
            )
            .expect("h2"),
            SubstanceDefinition::gas(
                SubstanceId::parse("flux.default.substance.o2").expect("id"),
                plugin_id,
                "O2",
                31.998,
                [0.0, 0.85, 0.85],
                vec!["o2".to_string()],
            )
            .expect("o2"),
        ])
        .expect("registry");

        let ids = registry
            .all()
            .iter()
            .map(|definition| definition.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            vec![
                "flux.default.substance.h2",
                "flux.default.substance.o2",
                "flux.default.substance.co2"
            ]
        );
        assert_eq!(registry.compact_index("h2"), Some(0));
        assert_eq!(
            registry.compact_index("flux.default.substance.co2"),
            Some(2)
        );
    }
}
