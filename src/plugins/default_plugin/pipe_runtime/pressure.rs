use super::PipeSimulationConfig;

/// Converts world-cell particles into total pressure in pascals.
pub fn world_pressure_pa(config: &PipeSimulationConfig, world_particles: u32) -> f32 {
    world_particles as f32 * config.cell_particle_pressure_pa.max(0.0)
}

/// Converts pipe-node particles into total pressure in pascals.
pub fn pipe_pressure_pa(config: &PipeSimulationConfig, pipe_particles: u32) -> f32 {
    pipe_particles as f32
        * config.cell_particle_pressure_pa.max(0.0)
        * config.cell_volume_ratio.max(0.0)
}

/// Formats pressure in human-readable SI units for HUD usage.
pub fn format_pressure_pa(pressure_pa: f32) -> String {
    let clamped = pressure_pa.max(0.0);
    let units = [
        ("Pa", 1.0f32),
        ("kPa", 1_000.0f32),
        ("MPa", 1_000_000.0f32),
        ("GPa", 1_000_000_000.0f32),
    ];
    let mut selected = units[0];
    for candidate in units {
        let scaled = clamped / candidate.1;
        selected = candidate;
        if scaled < 1_000.0 {
            break;
        }
    }
    let scaled = clamped / selected.1;
    if scaled >= 999.5 || (scaled - scaled.round()).abs() <= 0.05 {
        format!("{scaled:.0}{}", selected.0)
    } else {
        format!("{scaled:.1}{}", selected.0)
    }
}

#[cfg(test)]
mod tests {
    use super::PipeSimulationConfig;
    use super::{format_pressure_pa, pipe_pressure_pa, world_pressure_pa};

    #[test]
    fn default_pressure_conversions_match_design() {
        let config = PipeSimulationConfig::default();
        assert_eq!(world_pressure_pa(&config, 1), 0.2);
        assert_eq!(pipe_pressure_pa(&config, 1), 5.0);
        assert_eq!(pipe_pressure_pa(&config, 500), 2_500.0);
    }

    #[test]
    fn pressure_formatter_uses_compact_si_units() {
        assert_eq!(format_pressure_pa(101_300.0), "101.3kPa");
        assert_eq!(format_pressure_pa(12_500_000.0), "12.5MPa");
        assert_eq!(format_pressure_pa(999.6), "1000Pa");
        assert_eq!(format_pressure_pa(1_250_000_000.0), "1.2GPa");
    }
}
