use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::Resource;

use crate::plugins::{
    api::{
        events::PluginEvent,
        render_api::OverlayRenderPolicy,
        ui_api::{PanelDescriptor, ToolDescriptor},
    },
    default_plugin, ContentId, LoadedPluginMetadata, PluginId,
};

const OVERLAY_HOTKEY_SLOTS: &[&str] = &[
    "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
];

/// Subscription declared by one plugin for runtime events.
///
/// # Fields
/// - `plugin_id`: Plugin that subscribed to the runtime event.
/// - `event_kind`: Event category the plugin asked to receive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginSubscription {
    pub plugin_id: PluginId,
    pub event_kind: PluginEvent,
}

/// Overlay descriptor registered through the plugin API.
///
/// # Fields
/// - `id`: Stable content id of the overlay.
/// - `plugin_id`: Plugin that owns this overlay descriptor.
/// - `label`: Human-readable overlay label shown in UI.
/// - `hotkey`: Optional hotkey assigned to activate the overlay.
/// - `render_policy`: Whether the overlay is rendered by core systems or by the plugin itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeOverlayDescriptor {
    pub id: ContentId,
    pub plugin_id: PluginId,
    pub label: String,
    pub hotkey: Option<String>,
    pub render_policy: OverlayRenderPolicy,
}

/// Save chunk descriptor registered through the plugin API.
///
/// # Fields
/// - `id`: Stable content id of the save chunk.
/// - `plugin_id`: Plugin that owns the save chunk namespace.
/// - `version`: Schema version written into saved chunk payloads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveChunkDescriptor {
    pub id: ContentId,
    pub plugin_id: PluginId,
    pub version: u32,
}

/// Runtime registry for plugin API descriptors and event subscriber groups.
///
/// # Fields
/// - `subscriptions`: Plugins grouped by event kind for runtime dispatch.
/// - `tools`: Registered plugin tool descriptors keyed by stable content id.
/// - `panels`: Registered plugin panel descriptors keyed by stable content id.
/// - `overlays`: Registered plugin overlay descriptors keyed by stable content id.
/// - `save_chunks`: Registered plugin save chunk descriptors keyed by stable content id.
#[derive(Resource, Clone, Debug, Default)]
pub struct PluginRuntimeRegistry {
    subscriptions: BTreeMap<PluginEvent, BTreeSet<PluginId>>,
    tools: BTreeMap<ContentId, ToolDescriptor>,
    panels: BTreeMap<ContentId, PanelDescriptor>,
    overlays: BTreeMap<ContentId, RuntimeOverlayDescriptor>,
    save_chunks: BTreeMap<ContentId, SaveChunkDescriptor>,
}

impl PluginRuntimeRegistry {
    /// Registers one plugin as a subscriber for the given event kind.
    ///
    pub fn subscribe(&mut self, plugin_id: PluginId, event_kind: PluginEvent) {
        self.subscriptions
            .entry(event_kind)
            .or_default()
            .insert(plugin_id);
    }

    /// Returns plugin ids subscribed to one event kind.
    ///
    pub fn subscribers(&self, event_kind: PluginEvent) -> Vec<PluginId> {
        self.subscriptions
            .get(&event_kind)
            .map(|plugins| plugins.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Registers a plugin-owned tool descriptor.
    ///
    pub fn register_tool(&mut self, descriptor: ToolDescriptor) {
        self.tools.insert(descriptor.id.clone(), descriptor);
    }

    /// Registers a plugin-owned panel descriptor.
    ///
    pub fn register_panel(&mut self, descriptor: PanelDescriptor) {
        self.panels.insert(descriptor.id.clone(), descriptor);
    }

    /// Registers a plugin-owned overlay descriptor.
    ///
    pub fn register_overlay(&mut self, descriptor: RuntimeOverlayDescriptor) {
        self.overlays.insert(descriptor.id.clone(), descriptor);
    }

    /// Registers a plugin-owned save chunk descriptor.
    ///
    pub fn register_save_chunk(&mut self, descriptor: SaveChunkDescriptor) {
        self.save_chunks.insert(descriptor.id.clone(), descriptor);
    }

    /// Returns registered tool descriptors.
    ///
    pub fn tools(&self) -> &BTreeMap<ContentId, ToolDescriptor> {
        &self.tools
    }

    /// Returns registered panel descriptors.
    ///
    pub fn panels(&self) -> &BTreeMap<ContentId, PanelDescriptor> {
        &self.panels
    }

    /// Returns registered overlay descriptors.
    ///
    pub fn overlays(&self) -> &BTreeMap<ContentId, RuntimeOverlayDescriptor> {
        &self.overlays
    }

    /// Returns registered save chunk descriptors.
    ///
    pub fn save_chunks(&self) -> &BTreeMap<ContentId, SaveChunkDescriptor> {
        &self.save_chunks
    }
}

/// Builds the runtime API registry from enabled plugin metadata.
///
pub fn build_plugin_runtime_registry(
    loaded_plugins: &[LoadedPluginMetadata],
) -> PluginRuntimeRegistry {
    let mut registry = PluginRuntimeRegistry::default();
    let mut used_hotkeys = BTreeSet::new();
    for descriptor in default_plugin::default_overlay_descriptors() {
        used_hotkeys.insert(descriptor.hotkey.to_string());
        registry.register_overlay(RuntimeOverlayDescriptor {
            id: descriptor.id,
            plugin_id: descriptor.plugin_id,
            label: descriptor.label.to_string(),
            hotkey: Some(descriptor.hotkey.to_string()),
            render_policy: OverlayRenderPolicy::CoreDefault,
        });
    }
    for plugin in loaded_plugins {
        for subscription in &plugin.registration.subscriptions {
            registry.subscribe(plugin.plugin_id.clone(), subscription.event_kind);
        }
        for panel in &plugin.registration.panels {
            registry.register_panel(panel.clone());
        }
        for tool in &plugin.registration.tools {
            registry.register_tool(tool.clone());
        }
        for overlay in &plugin.registration.overlays {
            let mut overlay = overlay.clone();
            overlay.hotkey = overlay
                .hotkey
                .filter(|hotkey| !hotkey.trim().is_empty())
                .or_else(|| next_overlay_hotkey(&used_hotkeys));
            if let Some(hotkey) = &overlay.hotkey {
                used_hotkeys.insert(hotkey.clone());
            }
            registry.register_overlay(overlay);
        }
        for save_chunk in &plugin.registration.save_chunks {
            registry.register_save_chunk(save_chunk.clone());
        }
    }
    registry
}

fn next_overlay_hotkey(used_hotkeys: &BTreeSet<String>) -> Option<String> {
    OVERLAY_HOTKEY_SLOTS
        .iter()
        .find(|slot| !used_hotkeys.contains(**slot))
        .map(|slot| (*slot).to_string())
}

#[cfg(test)]
mod tests {
    use super::{build_plugin_runtime_registry, PluginRuntimeRegistry};
    use crate::plugins::{
        api::{events::PluginEvent, render_api::OverlayRenderPolicy},
        LoadedPluginMetadata, PluginId, PluginRuntimeRegistration, RuntimeOverlayDescriptor,
    };

    #[test]
    fn subscriptions_are_grouped_by_event_kind() {
        let plugin = PluginId::parse("flux.test").expect("plugin id");
        let mut registry = PluginRuntimeRegistry::default();
        registry.subscribe(plugin.clone(), PluginEvent::MouseDownCell);
        registry.subscribe(plugin.clone(), PluginEvent::MouseDownCell);

        assert_eq!(
            registry.subscribers(PluginEvent::MouseDownCell),
            vec![plugin]
        );
        assert!(registry.subscribers(PluginEvent::MouseUpCell).is_empty());
    }

    #[test]
    fn runtime_overlays_get_next_free_hotkey_after_default_overlays() {
        let plugin_id = PluginId::parse("flux.test").expect("plugin id");
        let overlay_id =
            crate::plugins::ContentId::parse("flux.test.overlay.temperature").expect("overlay id");
        let registry = build_plugin_runtime_registry(&[LoadedPluginMetadata {
            plugin_id: plugin_id.clone(),
            display_name: "Test".to_string(),
            version: crate::plugins::PluginVersion::parse("0.1.0").expect("version"),
            source_kind: crate::plugins::PluginSourceKind::Dev,
            content: false,
            locked: false,
            source_name: "flux.test".to_string(),
            source_path: None,
            manifest: None,
            registration: PluginRuntimeRegistration {
                overlays: vec![RuntimeOverlayDescriptor {
                    id: overlay_id.clone(),
                    plugin_id,
                    label: "Temperature".to_string(),
                    hotkey: None,
                    render_policy: OverlayRenderPolicy::PluginControlled,
                }],
                ..Default::default()
            },
        }]);

        assert_eq!(
            registry
                .overlays()
                .get(&overlay_id)
                .expect("overlay")
                .hotkey
                .as_deref(),
            Some("F4")
        );
    }
}
