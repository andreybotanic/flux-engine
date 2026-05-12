use crate::plugins::{
    api::{
        events::PluginEvent,
        runtime::{RuntimeOverlayDescriptor, SaveChunkDescriptor},
        ui_api::{PanelDescriptor, ToolDescriptor},
    },
    SubstanceDefinition,
};

/// One runtime event subscription declared by a plugin.
///
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginSubscriptionRegistration {
    pub event_kind: PluginEvent,
}

/// Runtime content registered by a plugin during the ABI handshake.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PluginRuntimeRegistration {
    pub gas_substances: Vec<SubstanceDefinition>,
    pub entities: Vec<flux_plugin_sdk::EntityDescriptor>,
    pub subscriptions: Vec<PluginSubscriptionRegistration>,
    pub tools: Vec<ToolDescriptor>,
    pub panels: Vec<PanelDescriptor>,
    pub overlays: Vec<RuntimeOverlayDescriptor>,
    pub save_chunks: Vec<SaveChunkDescriptor>,
}

impl PluginRuntimeRegistration {
    /// Returns `true` when the plugin did not register any content.
    pub fn is_empty(&self) -> bool {
        self.gas_substances.is_empty()
            && self.entities.is_empty()
            && self.subscriptions.is_empty()
            && self.tools.is_empty()
            && self.panels.is_empty()
            && self.overlays.is_empty()
            && self.save_chunks.is_empty()
    }
}
