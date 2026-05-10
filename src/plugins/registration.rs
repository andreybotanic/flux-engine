use crate::plugins::SubstanceDefinition;

/// Runtime content registered by a plugin during the ABI handshake.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PluginRuntimeRegistration {
    pub gas_substances: Vec<SubstanceDefinition>,
}

impl PluginRuntimeRegistration {
    /// Returns `true` when the plugin did not register any content.
    pub fn is_empty(&self) -> bool {
        self.gas_substances.is_empty()
    }
}
