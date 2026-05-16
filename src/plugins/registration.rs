use crate::plugins::{
    api::{
        events::PluginEvent,
        runtime::{RuntimeOverlayDescriptor, SaveChunkDescriptor},
        ui_api::EntityCategoryDescriptor,
        ui_api::ToolDescriptor,
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
    pub entity_categories: Vec<EntityCategoryDescriptor>,
    pub entities: Vec<flux_plugin_sdk::EntityDescriptor>,
    pub subscriptions: Vec<PluginSubscriptionRegistration>,
    pub tools: Vec<ToolDescriptor>,
    pub overlays: Vec<RuntimeOverlayDescriptor>,
    pub overlay_materials: Vec<flux_plugin_sdk::OverlayMaterialDescriptor>,
    pub save_chunks: Vec<SaveChunkDescriptor>,
}

impl PluginRuntimeRegistration {
    /// Returns `true` when the plugin did not register any content.
    pub fn is_empty(&self) -> bool {
        self.gas_substances.is_empty()
            && self.entity_categories.is_empty()
            && self.entities.is_empty()
            && self.subscriptions.is_empty()
            && self.tools.is_empty()
            && self.overlays.is_empty()
            && self.overlay_materials.is_empty()
            && self.save_chunks.is_empty()
    }
}
