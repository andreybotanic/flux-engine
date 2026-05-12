use flux_plugin_sdk::{declare_plugin, Plugin, PluginError, PluginInit, Registrar};

/// Minimal non-content sample plugin used by packaging and loader smoke tests.
pub struct Stage1SamplePlugin;

impl Plugin for Stage1SamplePlugin {
    fn new(_init: PluginInit) -> Result<Self, PluginError> {
        Ok(Self)
    }

    fn register(&mut self, _registrar: &mut Registrar<Self>) -> Result<(), PluginError> {
        Ok(())
    }
}

declare_plugin!(Stage1SamplePlugin);
