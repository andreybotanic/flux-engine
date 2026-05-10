use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    time::{Instant, SystemTime},
};

use bevy::prelude::Vec2;
use flux_engine::{
    config::GasRegistry,
    plugins::default_plugin::default_substance_definitions,
    simulation::{
        backend::SimulationBackend,
        discrete_step::{step_discrete_in_place, DiscreteStepParams},
        gpu_solver::{GpuGasSolver, GpuSolverHostState},
        parity::run_full_parity_gate,
        GasSimulationConfig, SolverTuning,
    },
};

const PERF_SCENARIO_PROFILE: &str = "app_like_sparse";

fn perf_registry() -> Result<GasRegistry, String> {
    GasRegistry::from_substances(default_substance_definitions())
}

#[derive(Clone, Debug)]
struct PerfSample {
    backend: SimulationBackend,
    world_w: u32,
    world_h: u32,
    mode: &'static str,
    run: u32,
    run_started_at_utc: String,
    step_compute_ms: f32,
    upload_to_gpu_ms: f32,
    readback_from_gpu_ms: f32,
    step_total_ms: f32,
}

#[derive(Clone)]
struct DiscretePerfState {
    width: u32,
    height: u32,
    step: u64,
    gas_count: usize,
    molecular_masses: Vec<f32>,
    read: Vec<u32>,
    write: Vec<u32>,
    total_density: Vec<f32>,
    velocity: Vec<Vec2>,
    solid_mask: Vec<u32>,
}

impl DiscretePerfState {
    fn seeded(width: u32, height: u32, registry: &GasRegistry) -> Self {
        let cells = (width * height) as usize;
        let molecular_masses = registry.molecular_masses();
        let gas_count = molecular_masses.len();
        let mut state = Self {
            width,
            height,
            step: 0,
            gas_count,
            molecular_masses,
            read: vec![0; cells * gas_count],
            write: vec![0; cells * gas_count],
            total_density: vec![0.0; cells],
            velocity: vec![Vec2::ZERO; cells],
            solid_mask: vec![0; cells],
        };

        for y in 0..height {
            for x in 0..width {
                let idx = state.idx(x, y);
                if state.is_boundary(x, y) {
                    state.solid_mask[idx] = 1;
                }
            }
        }

        let cx = width / 2;
        let cy = height / 2;
        let rx = (width / 10).max(6);
        let ry = (height / 10).max(6);
        for y in cy.saturating_sub(ry)..=(cy + ry).min(height - 2) {
            for x in cx.saturating_sub(rx)..=(cx + rx).min(width - 2) {
                if state.is_boundary(x, y) || state.is_solid(x, y) {
                    continue;
                }
                let checker = (x + y) % 4;
                if state.gas_count > 0 && checker <= 1 {
                    state.set_cell_species(x, y, 0, 180 + ((x * 7 + y * 13) % 40));
                }
                if state.gas_count > 1 && (checker == 1 || checker == 2) {
                    state.set_cell_species(x, y, 1, 90 + ((x * 11 + y * 5) % 25));
                }
                if state.gas_count > 2 && (checker == 2 || checker == 3) {
                    state.set_cell_species(x, y, 2, 45 + ((x * 3 + y * 17) % 20));
                }
            }
        }

        state.recompute_macro_fields();
        state.write.copy_from_slice(&state.read);
        state
    }

    fn idx(&self, x: u32, y: u32) -> usize {
        (y * self.width + x) as usize
    }

    fn species_offset(&self, x: u32, y: u32, gas_index: usize) -> usize {
        self.idx(x, y) * self.gas_count + gas_index
    }

    fn is_boundary(&self, x: u32, y: u32) -> bool {
        x == 0 || y == 0 || x == self.width - 1 || y == self.height - 1
    }

    fn is_solid(&self, x: u32, y: u32) -> bool {
        self.solid_mask[self.idx(x, y)] != 0
    }

    fn set_cell_species(&mut self, x: u32, y: u32, gas_index: usize, value: u32) {
        let offset = self.species_offset(x, y, gas_index);
        self.read[offset] = value;
        self.write[offset] = value;
    }

    fn recompute_macro_fields(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = self.idx(x, y);
                if self.is_boundary(x, y) || self.is_solid(x, y) {
                    self.total_density[idx] = 0.0;
                    self.velocity[idx] = Vec2::ZERO;
                    continue;
                }
                let base = idx * self.gas_count;
                self.total_density[idx] = self.read[base..base + self.gas_count]
                    .iter()
                    .map(|v| *v as f32)
                    .sum();
                self.velocity[idx] = Vec2::ZERO;
            }
        }
    }

    fn to_gpu_host_state(&self) -> GpuSolverHostState {
        let cells = (self.width * self.height) as usize;
        let mut species = Vec::with_capacity(cells * self.gas_count);
        for idx in 0..cells {
            let base = idx * self.gas_count;
            species.extend_from_slice(&self.read[base..base + self.gas_count]);
        }
        GpuSolverHostState {
            gas_count: self.gas_count as u32,
            molecular_masses: self.molecular_masses.clone(),
            species,
            total_density: self.total_density.clone(),
            velocity: self.velocity.iter().map(|v| [v.x, v.y]).collect(),
            solid_mask: self.solid_mask.clone(),
        }
    }

    fn do_one_substep(&mut self, config: &GasSimulationConfig) {
        let width = self.width;
        let solid_mask = &self.solid_mask;
        let solid_query = |x: u32, y: u32| solid_mask[(y * width + x) as usize] != 0;
        step_discrete_in_place(DiscreteStepParams {
            width: self.width,
            height: self.height,
            gas_count: self.gas_count,
            molecular_masses: &self.molecular_masses,
            read: &mut self.read,
            write: &mut self.write,
            total_density: &mut self.total_density,
            velocity: &mut self.velocity,
            solid_query: &solid_query,
            temperature_multiplier: |_x, _y| 1.0,
            tuning: &config.solver_tuning,
            thermal_motion_scale: config.thermal_motion_scale,
            simulation_step: self.step,
        });
        self.step = self.step.saturating_add(1);
    }
}

fn percentile(values: &[f32], pct: f32) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((sorted.len() - 1) as f32 * pct).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn mean(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f32>() / values.len() as f32
}

fn estimate_mode_seconds(step_ms: f32, warmup: u32, measure_steps: u32, repeats: u32) -> f32 {
    let steps_total = (warmup + measure_steps) as f32 * repeats as f32;
    step_ms * steps_total / 1000.0
}

fn should_abort_preflight(
    allow_slow: bool,
    estimated_total_seconds: f32,
    max_estimated_seconds: f32,
    warnings: &[String],
    blockers: &[String],
) -> bool {
    !allow_slow
        && (estimated_total_seconds > max_estimated_seconds
            || !warnings.is_empty()
            || !blockers.is_empty())
}

fn ensure_required_sizes(world_sizes: &[(u32, u32)]) -> Result<(), String> {
    let need_502 = world_sizes.iter().any(|(w, h)| *w == 502 && *h == 502);
    let need_1002 = world_sizes.iter().any(|(w, h)| *w == 1002 && *h == 1002);
    if need_502 && need_1002 {
        Ok(())
    } else {
        Err("Performance test must include both required sizes: 502x502 and 1002x1002.".to_string())
    }
}

fn run_cpu(
    width: u32,
    height: u32,
    warmup: u32,
    measure_steps: u32,
    repeats: u32,
    config: &GasSimulationConfig,
    registry: &GasRegistry,
    run_started_at_utc: &str,
) -> Vec<PerfSample> {
    let mut out = Vec::new();

    for run in 0..repeats {
        let mut state = DiscretePerfState::seeded(width, height, registry);
        for _ in 0..warmup {
            state.do_one_substep(config);
        }

        let mut step_compute = Vec::with_capacity(measure_steps as usize);
        for _ in 0..measure_steps {
            let started = Instant::now();
            state.do_one_substep(config);
            let ms = started.elapsed().as_secs_f32() * 1000.0;
            step_compute.push(ms);
        }

        out.push(PerfSample {
            backend: SimulationBackend::Cpu,
            world_w: width,
            world_h: height,
            mode: "runtime_transfer",
            run,
            run_started_at_utc: run_started_at_utc.to_string(),
            step_compute_ms: mean(&step_compute),
            upload_to_gpu_ms: 0.0,
            readback_from_gpu_ms: 0.0,
            step_total_ms: mean(&step_compute),
        });
    }

    out
}

fn run_gpu_mode(
    width: u32,
    height: u32,
    warmup: u32,
    measure_steps: u32,
    repeats: u32,
    config: &GasSimulationConfig,
    registry: &GasRegistry,
    mode_name: &'static str,
    run_started_at_utc: &str,
) -> (Vec<PerfSample>, String) {
    let mut out = Vec::new();
    let mut adapter_text = String::new();

    for run in 0..repeats {
        let mut state = DiscretePerfState::seeded(width, height, registry);
        let mut solver =
            match GpuGasSolver::new_with_gas_count(width, height, state.gas_count as u32) {
                Ok(solver) => solver,
                Err(err) => {
                    eprintln!("GPU is unavailable for {width}x{height} ({mode_name}): {err}");
                    return (out, "GPU unavailable".to_string());
                }
            };
        let info = solver.adapter_info();
        adapter_text = format!(
            "{} | backend={} | vendor={} | device={} | driver={} | driver_info={}",
            info.name, info.backend, info.vendor, info.device, info.driver, info.driver_info
        );

        let upload_ms = match solver.upload_state(&state.to_gpu_host_state()) {
            Ok(v) => v,
            Err(err) => {
                eprintln!("Upload failed for {width}x{height}: {err}");
                continue;
            }
        };

        for _ in 0..warmup {
            let params = GpuGasSolver::params_from_raw(
                config,
                width,
                height,
                state.step,
                state.gas_count as u32,
            );
            let _ = solver.step(params);
            state.step = state.step.saturating_add(1);
        }

        let mut compute_samples = Vec::with_capacity(measure_steps as usize);
        let mut readback_samples = Vec::with_capacity(measure_steps as usize);
        let mut total_samples = Vec::with_capacity(measure_steps as usize);

        for _ in 0..measure_steps {
            let params = GpuGasSolver::params_from_raw(
                config,
                width,
                height,
                state.step,
                state.gas_count as u32,
            );
            let timings = match solver.step(params) {
                Ok(t) => t,
                Err(err) => {
                    eprintln!("GPU step failed for {width}x{height}: {err}");
                    break;
                }
            };
            state.step = state.step.saturating_add(1);
            compute_samples.push(timings.compute_gpu_ms);
            readback_samples.push(timings.readback_from_gpu_ms);
            total_samples.push(timings.step_total_ms);
        }

        if compute_samples.is_empty() {
            continue;
        }

        let upload_per_step = upload_ms / measure_steps as f32;
        out.push(PerfSample {
            backend: SimulationBackend::Gpu,
            world_w: width,
            world_h: height,
            mode: mode_name,
            run,
            run_started_at_utc: run_started_at_utc.to_string(),
            step_compute_ms: mean(&compute_samples),
            upload_to_gpu_ms: upload_per_step,
            readback_from_gpu_ms: mean(&readback_samples),
            step_total_ms: mean(&total_samples) + upload_per_step,
        });
    }

    (out, adapter_text)
}

fn write_csv(path: &Path, samples: &[PerfSample]) -> Result<(), String> {
    let mut out = String::new();
    out.push_str("backend,world_w,world_h,mode,run,run_started_at_utc,step_compute_ms,upload_to_gpu_ms,readback_from_gpu_ms,step_total_ms\n");
    for s in samples {
        let backend = match s.backend {
            SimulationBackend::Cpu => "cpu",
            SimulationBackend::Gpu => "gpu",
        };
        out.push_str(&format!(
            "{backend},{},{},{},{},{},{:.6},{:.6},{:.6},{:.6}\n",
            s.world_w,
            s.world_h,
            s.mode,
            s.run,
            s.run_started_at_utc,
            s.step_compute_ms,
            s.upload_to_gpu_ms,
            s.readback_from_gpu_ms,
            s.step_total_ms
        ));
    }
    fs::write(path, out).map_err(|e| format!("Failed to write CSV: {e}"))
}

fn write_markdown(path: &Path, samples: &[PerfSample], adapter: &str) -> Result<(), String> {
    let mut grouped: BTreeMap<(String, u32, u32, String), Vec<&PerfSample>> = BTreeMap::new();
    for sample in samples {
        let backend = match sample.backend {
            SimulationBackend::Cpu => "cpu".to_string(),
            SimulationBackend::Gpu => "gpu".to_string(),
        };
        grouped
            .entry((
                backend,
                sample.world_w,
                sample.world_h,
                sample.mode.to_string(),
            ))
            .or_default()
            .push(sample);
    }

    let run_started_at = samples
        .first()
        .map(|s| s.run_started_at_utc.as_str())
        .unwrap_or("n/a");

    let mut out = String::new();
    out.push_str("# Gas Simulation GPU Migration Perf Report\n\n");
    out.push_str("## Metadata\n\n");
    out.push_str(&format!("- Run started at (UTC): {}\n", run_started_at));
    out.push_str(&format!("- Scenario profile: {}\n", PERF_SCENARIO_PROFILE));
    out.push_str(&format!("- GPU adapter: {}\n", adapter));
    out.push_str(&format!("- Total samples: {}\n\n", samples.len()));

    out.push_str("## Summary (mean / p50 / p95)\n\n");
    out.push_str("| backend | size | mode | step_compute_ms (mean/p50/p95) | upload_to_gpu_ms (mean/p50/p95) | readback_from_gpu_ms (mean/p50/p95) | step_total_ms (mean/p50/p95) |\n");
    out.push_str("|---|---:|---|---:|---:|---:|---:|\n");

    for ((backend, w, h, mode), rows) in grouped {
        let c = rows.iter().map(|s| s.step_compute_ms).collect::<Vec<_>>();
        let u = rows.iter().map(|s| s.upload_to_gpu_ms).collect::<Vec<_>>();
        let r = rows
            .iter()
            .map(|s| s.readback_from_gpu_ms)
            .collect::<Vec<_>>();
        let t = rows.iter().map(|s| s.step_total_ms).collect::<Vec<_>>();
        out.push_str(&format!(
            "| {} | {}x{} | {} | {:.3}/{:.3}/{:.3} | {:.3}/{:.3}/{:.3} | {:.3}/{:.3}/{:.3} | {:.3}/{:.3}/{:.3} |\n",
            backend,
            w,
            h,
            mode,
            mean(&c),
            percentile(&c, 0.50),
            percentile(&c, 0.95),
            mean(&u),
            percentile(&u, 0.50),
            percentile(&u, 0.95),
            mean(&r),
            percentile(&r, 0.50),
            percentile(&r, 0.95),
            mean(&t),
            percentile(&t, 0.50),
            percentile(&t, 0.95)
        ));
    }

    fs::write(path, out).map_err(|e| format!("Failed to write markdown: {e}"))
}

fn main() -> Result<(), String> {
    if cfg!(debug_assertions) {
        return Err(
            "gas_perf must be run in release mode. Use: cargo run --release --bin gas_perf -- ..."
                .to_string(),
        );
    }

    let run_started_at_utc = humantime::format_rfc3339_millis(SystemTime::now()).to_string();
    let mut warmup = 200u32;
    let mut measure_steps = 500u32;
    let mut repeats = 5u32;
    let mut preflight_steps = 10u32;
    let mut skip_preflight = false;
    let mut preflight_only = false;
    let mut allow_slow = false;
    let mut max_estimated_seconds = 180.0f32;
    let mut world_sizes = vec![(102u32, 102u32), (502u32, 502u32), (1002u32, 1002u32)];
    let mut skip_parity_gate = false;

    let args: Vec<String> = std::env::args().collect();
    let mut i = 1usize;
    while i < args.len() {
        match args[i].as_str() {
            "--warmup" if i + 1 < args.len() => {
                if let Ok(v) = args[i + 1].parse::<u32>() {
                    warmup = v;
                }
                i += 1;
            }
            "--measure-steps" if i + 1 < args.len() => {
                if let Ok(v) = args[i + 1].parse::<u32>() {
                    measure_steps = v;
                }
                i += 1;
            }
            "--repeats" if i + 1 < args.len() => {
                if let Ok(v) = args[i + 1].parse::<u32>() {
                    repeats = v;
                }
                i += 1;
            }
            "--sizes" if i + 1 < args.len() => {
                let mut parsed = Vec::new();
                for item in args[i + 1].split(',') {
                    if let Some((w, h)) = item.split_once('x') {
                        if let (Ok(w), Ok(h)) = (w.parse::<u32>(), h.parse::<u32>()) {
                            parsed.push((w, h));
                        }
                    }
                }
                if !parsed.is_empty() {
                    world_sizes = parsed;
                }
                i += 1;
            }
            "--preflight-steps" if i + 1 < args.len() => {
                if let Ok(v) = args[i + 1].parse::<u32>() {
                    preflight_steps = v.clamp(1, 10);
                }
                i += 1;
            }
            "--skip-preflight" => {
                skip_preflight = true;
            }
            "--preflight-only" => {
                preflight_only = true;
            }
            "--allow-slow" => {
                allow_slow = true;
            }
            "--skip-parity-gate" => {
                skip_parity_gate = true;
            }
            "--max-estimated-seconds" if i + 1 < args.len() => {
                if let Ok(v) = args[i + 1].parse::<f32>() {
                    max_estimated_seconds = v.max(1.0);
                }
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }

    ensure_required_sizes(&world_sizes)?;

    let config = GasSimulationConfig {
        thermal_motion_scale: 0.08,
        solver_tuning: SolverTuning::default(),
        ..GasSimulationConfig::default()
    };
    let registry = perf_registry()?;

    if !skip_parity_gate {
        println!("Running parity gate before performance benchmark (2 scenarios x 5000 steps)...");
        let parity_results = run_full_parity_gate()?;
        for (scenario, metrics) in parity_results {
            println!(
                "Parity PASS [{}]: cells={}, mae={:.3}, p95={:.3}, max={:.3}, mass={:?}",
                scenario.name,
                metrics.compared_cells,
                metrics.mean_abs_error,
                metrics.p95_abs_error,
                metrics.max_abs_error,
                metrics.mass_rel_errors
            );
        }
    }

    if !skip_preflight {
        let pf_warmup = warmup.min(5);
        let pf_measure = preflight_steps.clamp(1, 10);
        let mut estimated_total_seconds = 0.0f32;
        let mut warnings = Vec::new();
        let mut blockers = Vec::new();
        let mut speedup_502 = None;
        let mut speedup_1002 = None;

        for (w, h) in &world_sizes {
            let cpu_pf = run_cpu(
                *w,
                *h,
                pf_warmup,
                pf_measure,
                1,
                &config,
                &registry,
                &run_started_at_utc,
            );
            let cpu_step = cpu_pf
                .first()
                .map(|s| s.step_total_ms)
                .ok_or_else(|| format!("Preflight failed for CPU {}x{}", w, h))?;
            estimated_total_seconds +=
                estimate_mode_seconds(cpu_step, warmup, measure_steps, repeats);

            let (gpu_runtime_pf, _) = run_gpu_mode(
                *w,
                *h,
                pf_warmup,
                pf_measure,
                1,
                &config,
                &registry,
                "runtime_transfer",
                &run_started_at_utc,
            );
            if let Some(sample) = gpu_runtime_pf.first() {
                estimated_total_seconds +=
                    estimate_mode_seconds(sample.step_total_ms, warmup, measure_steps, repeats);
                if sample.step_total_ms > cpu_step * 2.0 {
                    warnings.push(format!(
                        "GPU runtime step is {:.1}x slower than CPU on {}x{} (cpu={:.3}ms, gpu={:.3}ms)",
                        sample.step_total_ms / cpu_step.max(1e-6),
                        w,
                        h,
                        cpu_step,
                        sample.step_total_ms
                    ));
                }
                if *w == 502 && *h == 502 {
                    let speedup = cpu_step / sample.step_total_ms.max(1e-6);
                    speedup_502 = Some(speedup);
                    if sample.step_total_ms >= cpu_step {
                        blockers.push(format!(
                            "GPU runtime_transfer is not faster than CPU on 502x502 (cpu={:.3}ms, gpu={:.3}ms)",
                            cpu_step, sample.step_total_ms
                        ));
                    }
                }
                if *w == 1002 && *h == 1002 && sample.step_total_ms >= cpu_step {
                    blockers.push(format!(
                        "GPU runtime_transfer is not faster than CPU on 1002x1002 (cpu={:.3}ms, gpu={:.3}ms)",
                        cpu_step, sample.step_total_ms
                    ));
                }
                if *w == 1002 && *h == 1002 {
                    let speedup = cpu_step / sample.step_total_ms.max(1e-6);
                    speedup_1002 = Some(speedup);
                }
            } else {
                blockers.push(format!(
                    "GPU runtime_transfer preflight missing sample for {}x{}",
                    w, h
                ));
            }
        }

        if let (Some(s502), Some(s1002)) = (speedup_502, speedup_1002) {
            if s1002 <= s502 {
                blockers.push(format!(
                    "Speedup scaling requirement failed: speedup_1002={:.3} is not greater than speedup_502={:.3}",
                    s1002, s502
                ));
            }
        } else {
            blockers.push(
                "Preflight could not compute speedups for both required sizes (502x502 and 1002x1002)."
                    .to_string(),
            );
        }

        println!(
            "Preflight estimate: ~{:.1} sec for full run (warmup={}, measure_steps={}, repeats={})",
            estimated_total_seconds, warmup, measure_steps, repeats
        );
        if !warnings.is_empty() {
            println!("Preflight warnings:");
            for warning in &warnings {
                println!("- {warning}");
            }
        }
        if !blockers.is_empty() {
            println!("Preflight blockers:");
            for blocker in &blockers {
                println!("- {blocker}");
            }
        }
        if should_abort_preflight(
            allow_slow,
            estimated_total_seconds,
            max_estimated_seconds,
            &warnings,
            &blockers,
        ) {
            return Err(
                "Preflight detected suspiciously slow configuration; aborting long run. \
Use --allow-slow to run anyway after investigation."
                    .to_string(),
            );
        }
    }

    if preflight_only {
        println!("Preflight-only mode complete: full benchmark was intentionally skipped.");
        return Ok(());
    }

    let mut samples = Vec::new();
    let mut adapter_info = String::from("n/a");
    for (w, h) in world_sizes {
        samples.extend(run_cpu(
            w,
            h,
            warmup,
            measure_steps,
            repeats,
            &config,
            &registry,
            &run_started_at_utc,
        ));

        let (gpu_runtime, adapter) = run_gpu_mode(
            w,
            h,
            warmup,
            measure_steps,
            repeats,
            &config,
            &registry,
            "runtime_transfer",
            &run_started_at_utc,
        );
        if adapter != "GPU unavailable" {
            adapter_info = adapter.clone();
        }
        samples.extend(gpu_runtime);
    }

    let reports_dir = Path::new("reports");
    fs::create_dir_all(reports_dir).map_err(|e| format!("Failed to create reports dir: {e}"))?;
    let csv_path = reports_dir.join("gas_perf_report.csv");
    let md_path = reports_dir.join("gas_perf_report.md");
    write_csv(&csv_path, &samples)?;
    write_markdown(&md_path, &samples, &adapter_info)?;

    println!("Report written:");
    println!("- {}", csv_path.display());
    println!("- {}", md_path.display());

    let mut cpu_502 = None;
    let mut gpu_502 = None;
    let mut cpu_1002 = None;
    let mut gpu_1002 = None;
    for s in &samples {
        if s.mode != "runtime_transfer" {
            continue;
        }
        match (s.backend, s.world_w, s.world_h) {
            (SimulationBackend::Cpu, 502, 502) => cpu_502 = Some(s.step_total_ms),
            (SimulationBackend::Gpu, 502, 502) => gpu_502 = Some(s.step_total_ms),
            (SimulationBackend::Cpu, 1002, 1002) => cpu_1002 = Some(s.step_total_ms),
            (SimulationBackend::Gpu, 1002, 1002) => gpu_1002 = Some(s.step_total_ms),
            _ => {}
        }
    }
    let (cpu_502, gpu_502, cpu_1002, gpu_1002) = (
        cpu_502.ok_or_else(|| "Missing CPU sample for 502x502".to_string())?,
        gpu_502.ok_or_else(|| "Missing GPU sample for 502x502".to_string())?,
        cpu_1002.ok_or_else(|| "Missing CPU sample for 1002x1002".to_string())?,
        gpu_1002.ok_or_else(|| "Missing GPU sample for 1002x1002".to_string())?,
    );
    let speedup_502 = cpu_502 / gpu_502.max(1e-6);
    let speedup_1002 = cpu_1002 / gpu_1002.max(1e-6);
    if !(speedup_502 > 1.0 && speedup_1002 > 1.0 && speedup_1002 > speedup_502) {
        return Err(format!(
            "Performance acceptance failed: speedup_502={:.3}, speedup_1002={:.3}. \
Require GPU faster than CPU on both sizes and speedup_1002 > speedup_502.",
            speedup_502, speedup_1002
        ));
    }
    println!(
        "Performance acceptance PASS: speedup_502={:.3}, speedup_1002={:.3}",
        speedup_502, speedup_1002
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::should_abort_preflight;

    #[test]
    fn preflight_aborts_when_gpu_not_faster_or_eta_too_high() {
        let warnings = vec!["gpu too slow".to_string()];
        let blockers = vec!["gpu not faster on 1002x1002".to_string()];

        assert!(should_abort_preflight(
            false, 250.0, 180.0, &warnings, &blockers
        ));
        assert!(should_abort_preflight(false, 90.0, 180.0, &warnings, &[]));
        assert!(should_abort_preflight(false, 90.0, 180.0, &[], &blockers));
        assert!(!should_abort_preflight(false, 90.0, 180.0, &[], &[]));
        assert!(!should_abort_preflight(
            true, 250.0, 180.0, &warnings, &blockers
        ));
    }
}
