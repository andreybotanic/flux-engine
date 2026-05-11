use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::Resource;

use crate::plugins::{
    api::{
        events::PluginEventKind,
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
/// Public fields of `PluginSubscription` are part of the generated SDK reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginSubscription {
    pub plugin_id: PluginId,
    pub event_kind: PluginEventKind,
}

/// Overlay descriptor registered through the plugin API.
///
/// # Fields
/// Public fields of `RuntimeOverlayDescriptor` are part of the generated SDK reference.
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
/// Public fields of `SaveChunkDescriptor` are part of the generated SDK reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveChunkDescriptor {
    pub id: ContentId,
    pub plugin_id: PluginId,
    pub version: u32,
}

/// Runtime registry for plugin API descriptors and event subscriber groups.
///
/// # Fields
/// Public fields of `PluginRuntimeRegistry` are part of the generated SDK reference.
#[derive(Resource, Clone, Debug, Default)]
pub struct PluginRuntimeRegistry {
    subscriptions: BTreeMap<PluginEventKind, BTreeSet<PluginId>>,
    tools: BTreeMap<ContentId, ToolDescriptor>,
    panels: BTreeMap<ContentId, PanelDescriptor>,
    overlays: BTreeMap<ContentId, RuntimeOverlayDescriptor>,
    save_chunks: BTreeMap<ContentId, SaveChunkDescriptor>,
}

impl PluginRuntimeRegistry {
    /// Registers one plugin as a subscriber for the given event kind.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `subscribe` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn subscribe(&mut self, plugin_id: PluginId, event_kind: PluginEventKind) {
        self.subscriptions
            .entry(event_kind)
            .or_default()
            .insert(plugin_id);
    }

    /// Returns plugin ids subscribed to one event kind.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `subscribers` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn subscribers(&self, event_kind: PluginEventKind) -> Vec<PluginId> {
        self.subscriptions
            .get(&event_kind)
            .map(|plugins| plugins.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Registers a plugin-owned tool descriptor.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `register_tool` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn register_tool(&mut self, descriptor: ToolDescriptor) {
        self.tools.insert(descriptor.id.clone(), descriptor);
    }

    /// Registers a plugin-owned panel descriptor.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `register_panel` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn register_panel(&mut self, descriptor: PanelDescriptor) {
        self.panels.insert(descriptor.id.clone(), descriptor);
    }

    /// Registers a plugin-owned overlay descriptor.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `register_overlay` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn register_overlay(&mut self, descriptor: RuntimeOverlayDescriptor) {
        self.overlays.insert(descriptor.id.clone(), descriptor);
    }

    /// Registers a plugin-owned save chunk descriptor.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `register_save_chunk` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn register_save_chunk(&mut self, descriptor: SaveChunkDescriptor) {
        self.save_chunks.insert(descriptor.id.clone(), descriptor);
    }

    /// Returns registered tool descriptors.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `tools` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn tools(&self) -> &BTreeMap<ContentId, ToolDescriptor> {
        &self.tools
    }

    /// Returns registered panel descriptors.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `panels` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn panels(&self) -> &BTreeMap<ContentId, PanelDescriptor> {
        &self.panels
    }

    /// Returns registered overlay descriptors.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `overlays` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn overlays(&self) -> &BTreeMap<ContentId, RuntimeOverlayDescriptor> {
        &self.overlays
    }

    /// Returns registered save chunk descriptors.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `save_chunks` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn save_chunks(&self) -> &BTreeMap<ContentId, SaveChunkDescriptor> {
        &self.save_chunks
    }
}

/// Builds the runtime API registry from enabled plugin metadata.
///
/// # SDK Example
/// ```rust
/// // Call `build_plugin_runtime_registry` from plugin-facing code when this operation is available in context.
/// ```
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
        for handler in &plugin.registration.event_handlers {
            registry.subscribe(plugin.plugin_id.clone(), handler.event_kind);
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
        api::{events::PluginEventKind, render_api::OverlayRenderPolicy},
        LoadedPluginMetadata, PluginId, PluginRuntimeRegistration, RuntimeOverlayDescriptor,
    };

    #[test]
    fn subscriptions_are_grouped_by_event_kind() {
        let plugin = PluginId::parse("flux.test").expect("plugin id");
        let mut registry = PluginRuntimeRegistry::default();
        registry.subscribe(plugin.clone(), PluginEventKind::MouseDownCell);
        registry.subscribe(plugin.clone(), PluginEventKind::MouseDownCell);

        assert_eq!(
            registry.subscribers(PluginEventKind::MouseDownCell),
            vec![plugin]
        );
        assert!(registry
            .subscribers(PluginEventKind::MouseUpCell)
            .is_empty());
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
