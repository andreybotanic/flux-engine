use crate::plugins::{
    api::{
        events::PluginEventKind,
        runtime::{RuntimeOverlayDescriptor, SaveChunkDescriptor},
        ui_api::ToolDescriptor,
    },
    SubstanceDefinition,
};

/// Runtime content registered by a plugin during the ABI handshake.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PluginRuntimeRegistration {
    pub gas_substances: Vec<SubstanceDefinition>,
    pub event_subscriptions: Vec<PluginEventKind>,
    pub tools: Vec<ToolDescriptor>,
    pub overlays: Vec<RuntimeOverlayDescriptor>,
    pub save_chunks: Vec<SaveChunkDescriptor>,
}

impl PluginRuntimeRegistration {
    /// Returns `true` when the plugin did not register any content.
    pub fn is_empty(&self) -> bool {
        self.gas_substances.is_empty()
            && self.event_subscriptions.is_empty()
            && self.tools.is_empty()
            && self.overlays.is_empty()
            && self.save_chunks.is_empty()
    }
}
