use std::{collections::BTreeMap, fs, path::Path, time::Instant};

use flux_engine::simulation::{
    backend::SimulationBackend,
    gas::GasField,
    gpu_solver::{GpuGasSolver, GpuTransferMode},
    perf_model::PerfGasState,
    GasSimulationConfig,
};

#[derive(Clone, Debug)]
struct PerfSample {
    backend: SimulationBackend,
    world_w: u32,
    world_h: u32,
    mode: &'static str,
    run: u32,
    step_compute_ms: f32,
    upload_to_gpu_ms: f32,
    readback_from_gpu_ms: f32,
    step_total_ms: f32,
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

fn run_cpu(
    width: u32,
    height: u32,
    warmup: u32,
    measure_steps: u32,
    repeats: u32,
    config: &GasSimulationConfig,
) -> Vec<PerfSample> {
    let mut out = Vec::new();

    for run in 0..repeats {
        let mut state = PerfGasState::seeded(width, height);
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
    transfer_mode: GpuTransferMode,
    mode_name: &'static str,
) -> (Vec<PerfSample>, String) {
    let fallback_gas = GasField::default();
    let mut out = Vec::new();
    let mut adapter_text = String::new();

    for run in 0..repeats {
        let mut cpu_state = PerfGasState::seeded(width, height);
        let mut solver = match GpuGasSolver::new(width, height) {
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

        let upload_ms = match solver.upload_state(&cpu_state.to_gpu_host_state()) {
            Ok(v) => v,
            Err(err) => {
                eprintln!("Upload failed for {width}x{height}: {err}");
                continue;
            }
        };

        for _ in 0..warmup {
            let params = GpuGasSolver::params_from_config(
                config,
                width,
                height,
                cpu_state.step,
                &fallback_gas,
            );
            let _ = solver.step(params, transfer_mode);
            cpu_state.step += 1;
        }

        let mut compute_samples = Vec::with_capacity(measure_steps as usize);
        let mut readback_samples = Vec::with_capacity(measure_steps as usize);
        let mut total_samples = Vec::with_capacity(measure_steps as usize);

        for _ in 0..measure_steps {
            let params = GpuGasSolver::params_from_config(
                config,
                width,
                height,
                cpu_state.step,
                &fallback_gas,
            );
            let timings = match solver.step(params, transfer_mode) {
                Ok(t) => t,
                Err(err) => {
                    eprintln!("GPU step failed for {width}x{height}: {err}");
                    break;
                }
            };
            cpu_state.step += 1;
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
    out.push_str("backend,world_w,world_h,mode,run,step_compute_ms,upload_to_gpu_ms,readback_from_gpu_ms,step_total_ms\n");
    for s in samples {
        let backend = match s.backend {
            SimulationBackend::Cpu => "cpu",
            SimulationBackend::Gpu => "gpu",
        };
        out.push_str(&format!(
            "{backend},{},{},{},{},{:.6},{:.6},{:.6},{:.6}\n",
            s.world_w,
            s.world_h,
            s.mode,
            s.run,
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

    let mut out = String::new();
    out.push_str("# Gas Simulation GPU Migration Perf Report\n\n");
    out.push_str("## Metadata\n\n");
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
    let mut warmup = 200u32;
    let mut measure_steps = 500u32;
    let mut repeats = 5u32;
    let mut preflight_steps = 10u32;
    let mut skip_preflight = false;
    let mut allow_slow = false;
    let mut max_estimated_seconds = 180.0f32;
    let mut world_sizes = vec![(102u32, 102u32), (502u32, 502u32), (1002u32, 1002u32)];

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
            "--allow-slow" => {
                allow_slow = true;
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

    let config = GasSimulationConfig::default();

    if !skip_preflight {
        let pf_warmup = warmup.min(5);
        let pf_measure = preflight_steps.clamp(1, 10);
        let mut estimated_total_seconds = 0.0f32;
        let mut warnings = Vec::new();
        let mut blockers = Vec::new();

        for (w, h) in &world_sizes {
            let cpu_pf = run_cpu(*w, *h, pf_warmup, pf_measure, 1, &config);
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
                GpuTransferMode::RuntimeTransfer,
                "runtime_transfer",
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
                if *w == 1002 && *h == 1002 && sample.step_total_ms >= cpu_step {
                    blockers.push(format!(
                        "GPU runtime_transfer is not faster than CPU on 1002x1002 (cpu={:.3}ms, gpu={:.3}ms)",
                        cpu_step, sample.step_total_ms
                    ));
                }
            } else {
                blockers.push(format!(
                    "GPU runtime_transfer preflight missing sample for {}x{}",
                    w, h
                ));
            }

            let (gpu_forced_pf, _) = run_gpu_mode(
                *w,
                *h,
                pf_warmup,
                pf_measure,
                1,
                &config,
                GpuTransferMode::ForcedFullReadback,
                "forced_full_readback",
            );
            if let Some(sample) = gpu_forced_pf.first() {
                estimated_total_seconds +=
                    estimate_mode_seconds(sample.step_total_ms, warmup, measure_steps, repeats);
            }
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

    let mut samples = Vec::new();
    let mut adapter_info = String::from("n/a");

    for (w, h) in world_sizes {
        samples.extend(run_cpu(w, h, warmup, measure_steps, repeats, &config));

        let (gpu_runtime, adapter) = run_gpu_mode(
            w,
            h,
            warmup,
            measure_steps,
            repeats,
            &config,
            GpuTransferMode::RuntimeTransfer,
            "runtime_transfer",
        );
        if adapter != "GPU unavailable" {
            adapter_info = adapter.clone();
        }
        samples.extend(gpu_runtime);

        let (gpu_forced, adapter_forced) = run_gpu_mode(
            w,
            h,
            warmup,
            measure_steps,
            repeats,
            &config,
            GpuTransferMode::ForcedFullReadback,
            "forced_full_readback",
        );
        if adapter_forced != "GPU unavailable" {
            adapter_info = adapter_forced;
        }
        samples.extend(gpu_forced);
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
