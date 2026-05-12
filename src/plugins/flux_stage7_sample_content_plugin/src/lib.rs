use flux_plugin_sdk::{
    declare_plugin, Plugin, PluginError, PluginInit, Registrar, SubstanceDescriptor, SubstanceId,
};

pub struct Stage7SampleContentPlugin;

impl Plugin for Stage7SampleContentPlugin {
    fn new(_init: PluginInit) -> Result<Self, PluginError> {
        Ok(Self)
    }

    fn register(&mut self, registrar: &mut Registrar<Self>) -> Result<(), PluginError> {
        registrar.register_substance(SubstanceDescriptor {
            id: SubstanceId::parse("flux.sample_content.substance.neon").map_err(PluginError::from)?,
            label: "Neon".to_string(),
            alias: "neon".to_string(),
            molecular_mass: 20.180,
            color: [1.0, 0.32, 0.78],
        })
    }
}

declare_plugin!(Stage7SampleContentPlugin);
