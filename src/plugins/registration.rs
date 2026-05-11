use crate::plugins::{
    api::{
        events::PluginEvent,
        runtime::{RuntimeOverlayDescriptor, SaveChunkDescriptor},
        ui_api::ToolDescriptor,
    },
    SubstanceDefinition,
};

/// One event handler explicitly registered by a runtime plugin.
///
/// # Fields
/// Public fields of `PluginEventHandlerRegistration` are part of the engine-side plugin runtime model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginEventHandlerRegistration {
    pub event_kind: PluginEvent,
    pub handler_name: String,
}

/// Runtime content registered by a plugin during the ABI handshake.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PluginRuntimeRegistration {
    pub gas_substances: Vec<SubstanceDefinition>,
    pub event_handlers: Vec<PluginEventHandlerRegistration>,
    pub tools: Vec<ToolDescriptor>,
    pub overlays: Vec<RuntimeOverlayDescriptor>,
    pub save_chunks: Vec<SaveChunkDescriptor>,
}

impl PluginRuntimeRegistration {
    /// Returns `true` when the plugin did not register any content.
    pub fn is_empty(&self) -> bool {
        self.gas_substances.is_empty()
            && self.event_handlers.is_empty()
            && self.tools.is_empty()
            && self.overlays.is_empty()
            && self.save_chunks.is_empty()
    }
}
