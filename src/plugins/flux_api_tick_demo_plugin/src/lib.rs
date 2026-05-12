use bevy_math::Vec2;
use flux_plugin_sdk::{
    declare_plugin, GasApi, Plugin, PluginError, PluginEvent, PluginInit, Registrar,
    SimulationPreCellGasStepEvent, SubstanceId,
};

const H2_SUBSTANCE_ID: &str = "h2";

pub struct ApiTickDemoPlugin {
    pub gases: GasApi,
    h2: SubstanceId,
}

impl Plugin for ApiTickDemoPlugin {
    fn new(init: PluginInit) -> Result<Self, PluginError> {
        Ok(Self {
            gases: init.gas_api(),
            h2: SubstanceId::parse(H2_SUBSTANCE_ID).map_err(PluginError::from)?,
        })
    }

    fn register(&mut self, registrar: &mut Registrar<Self>) -> Result<(), PluginError> {
        registrar.subscribe(
            PluginEvent::SimulationPreCellGasStep,
            Self::on_simulation_pre_cell_gas_step,
        )?;
        Ok(())
    }
}

impl ApiTickDemoPlugin {
    fn on_simulation_pre_cell_gas_step(
        &mut self,
        _event: &SimulationPreCellGasStepEvent,
    ) -> Result<(), PluginError> {
        let _ = self
            .gases
            .add_with_velocity((50, 50).into(), self.h2.clone(), 20, Vec2::new(3.0, 0.0))?;
        Ok(())
    }
}

declare_plugin!(ApiTickDemoPlugin);
