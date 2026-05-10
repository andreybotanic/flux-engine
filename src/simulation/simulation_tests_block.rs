#[cfg(test)]
mod tests {
    use crate::config::{GasDefinition, GasRegistry};
    use crate::plugins::default_plugin::pipe_runtime::{
        apply_gas_structures_pre_step, pipe_flow_reset_needed,
    };
    use crate::simulation::backend::{SimulationBackend, SimulationBackendConfig};
    use crate::simulation::gas::GasField;
    use crate::world::{grid::WorldGrid, structures::PlacedStructureMap};
    use super::{
        abort_on_gpu_runtime_error, effective_target_hz, SimulationRateConfig, SimulationSpeed,
    };

    fn test_registry() -> GasRegistry {
        GasRegistry::new(vec![
            GasDefinition {
                id: "h2".to_string(),
                label: "Hydrogen".to_string(),
                molecular_mass: 2.016,
                color: [0.6, 0.8, 1.0],
            },
            GasDefinition {
                id: "o2".to_string(),
                label: "Oxygen".to_string(),
                molecular_mass: 31.998,
                color: [0.6, 0.8, 1.0],
            },
            GasDefinition {
                id: "co2".to_string(),
                label: "Carbon Dioxide".to_string(),
                molecular_mass: 44.009,
                color: [0.9, 0.6, 0.4],
            },
        ])
        .expect("valid test registry")
    }

    #[test]
    fn default_backend_is_gpu() {
        assert_eq!(
            SimulationBackendConfig::default().backend,
            SimulationBackend::Gpu
        );
    }

    #[test]
    #[should_panic(expected = "GPU simulation backend failed")]
    fn gpu_runtime_error_policy_panics_and_aborts() {
        abort_on_gpu_runtime_error("synthetic failure");
    }

    #[test]
    fn effective_target_hz_scales_with_speed() {
        let base = 30;
        assert!((effective_target_hz(base, SimulationSpeed::X1) - 30.0).abs() <= f64::EPSILON);
        assert!((effective_target_hz(base, SimulationSpeed::X2) - 60.0).abs() <= f64::EPSILON);
        assert!((effective_target_hz(base, SimulationSpeed::X5) - 150.0).abs() <= f64::EPSILON);
    }

    #[test]
    fn effective_target_hz_clamps_base_rate_before_scaling() {
        let zero_base = SimulationRateConfig { target_hz: 0 };
        assert!((effective_target_hz(zero_base.target_hz, SimulationSpeed::X2) - 2.0).abs() <= f64::EPSILON);

        let huge_base = SimulationRateConfig { target_hz: 9_999 };
        assert!((effective_target_hz(huge_base.target_hz, SimulationSpeed::X5) - 5000.0).abs() <= f64::EPSILON);
    }

    #[test]
    fn pause_transition_requires_pipe_flow_reset_but_same_state_does_not() {
        assert!(!pipe_flow_reset_needed(None, true));
        assert!(!pipe_flow_reset_needed(Some(true), true));
        assert!(!pipe_flow_reset_needed(Some(false), false));
        assert!(pipe_flow_reset_needed(Some(true), false));
        assert!(pipe_flow_reset_needed(Some(false), true));
    }

    #[test]
    fn source_structure_adds_selected_gas_each_step() {
        let world = WorldGrid::default();
        let registry = test_registry();
        let mut gas = GasField::from_registry(&registry);
        let mut structures = PlacedStructureMap::default();
        assert!(structures.place_gas_source(10, 10, 1, 25, &world).is_some());

        let changed = apply_gas_structures_pre_step(&structures, &mut gas, &world);
        assert!(changed);
        assert_eq!(gas.amount_particles(10, 10, 0), 0);
        assert_eq!(gas.amount_particles(10, 10, 1), 25);
        assert_eq!(gas.total_amount_particles(10, 10), 25);
    }

    #[test]
    fn sink_structure_removes_proportionally_and_is_deterministic() {
        let world = WorldGrid::default();
        let registry = test_registry();
        let mut gas = GasField::from_registry(&registry);
        let mut structures = PlacedStructureMap::default();
        assert!(structures.place_gas_sink(12, 12, 10, &world).is_some());

        gas.set_amount(12, 12, 0, 10.0);
        gas.set_amount(12, 12, 1, 20.0);
        gas.set_amount(12, 12, 2, 30.0);
        gas.recompute_total_density_buffer(&world);

        let changed = apply_gas_structures_pre_step(&structures, &mut gas, &world);
        assert!(changed);
        assert_eq!(gas.total_amount_particles(12, 12), 50);
        assert_eq!(gas.amount_particles(12, 12, 0), 8);
        assert_eq!(gas.amount_particles(12, 12, 1), 17);
        assert_eq!(gas.amount_particles(12, 12, 2), 25);
    }
}
