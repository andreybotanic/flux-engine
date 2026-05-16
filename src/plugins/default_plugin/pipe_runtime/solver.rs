use std::collections::{HashMap, HashSet, VecDeque};

use super::PipeSimulationConfig;

use super::{
    pressure::{pipe_pressure_pa, world_pressure_pa},
    *,
};

#[derive(Clone, Copy, Debug)]
enum VentRequestDirection {
    WorldToPipe,
}

#[derive(Clone, Copy, Debug)]
struct DirectedPipeCandidate {
    source_index: usize,
    target_index: usize,
    demand_particles: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct UndirectedPipeEdge {
    a: usize,
    b: usize,
}

impl UndirectedPipeEdge {
    fn new(first: usize, second: usize) -> Self {
        if first <= second {
            Self {
                a: first,
                b: second,
            }
        } else {
            Self {
                a: second,
                b: first,
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct DirectedPipeRequest {
    source_index: usize,
    target_index: usize,
    requested_amount: u32,
}

#[derive(Clone, Debug)]
struct VentContext {
    node_index: usize,
    world_cells: Vec<UVec2>,
    world_pressure: f32,
}

#[derive(Clone, Debug)]
struct VentRequest {
    node_index: usize,
    requested_amount: u32,
    direction: VentRequestDirection,
    world_cells: Vec<UVec2>,
}

#[derive(Clone, Debug)]
struct VentBufferEntry {
    node_index: usize,
    world_cells: Vec<UVec2>,
    species_counts: Vec<u32>,
}

pub(super) fn apply_pipe_network_step(
    structures: &PlacedStructureMap,
    pipe_gas: &mut PipeGasField,
    pipe_flux: &mut PipeFluxField,
    gas: &mut GasField,
    world: &WorldGrid,
    visual_state: &mut PipeFlowVisualState,
    config: &PipeSimulationConfig,
) -> bool {
    let is_hop_tick = visual_state.begin_tick(config.pipe_step_interval_ticks);

    if pipe_gas.gas_count() == 0 {
        pipe_flux.clear_all();
        visual_state.reset_flow();
        return false;
    }

    pipe_gas.sync_to_structures(structures);
    let runtime = PipeRuntime::from_structures(structures, pipe_gas);
    if runtime.nodes.is_empty() {
        pipe_flux.clear_all();
        visual_state.reset_flow();
        return false;
    }
    pipe_flux.sync_to_runtime(&runtime, pipe_gas);

    if !is_hop_tick {
        return false;
    }

    visual_state.begin_hop_recording();
    let mut pipe_changed =
        commit_planned_pipe_transfers(&runtime, pipe_gas, &visual_state.previous_transfers);

    let species_before_vent = (0..pipe_gas.node_count())
        .map(|node_id| pipe_gas.node_species_counts(node_id))
        .collect::<Vec<_>>();
    let mut next_pipe_species = species_before_vent.clone();

    let vent_contexts = collect_vent_contexts(&runtime, gas, config);
    let mut vent_buffers = drain_vent_pipe_nodes_to_buffer(&vent_contexts, &mut next_pipe_species);
    let buffered_outlet_nodes = vent_buffers
        .iter()
        .map(|entry| entry.node_index)
        .collect::<HashSet<_>>();
    let intake_vent_world_pressures =
        build_vent_world_pressure_lut(runtime.nodes.len(), &vent_contexts);
    let intake_vent_offers =
        build_vent_pressure_offer_lut(&runtime, &intake_vent_world_pressures, config);
    let (intake_edge_demand_pa, intake_outgoing_request_counts) =
        build_edge_offer_demands(&runtime, &intake_vent_offers, config);
    let intake_allowed_direction = segment_allowed_direction_map(
        &runtime,
        &intake_edge_demand_pa,
        &intake_vent_world_pressures,
        config,
    );
    let forward_demand_intake_nodes =
        vent_nodes_with_forward_direction(&runtime, &intake_allowed_direction);
    let unblocked_intake_requests = build_vent_requests(
        &runtime,
        &vent_contexts,
        &intake_vent_offers,
        &intake_outgoing_request_counts,
        &forward_demand_intake_nodes,
        &HashSet::new(),
        &intake_allowed_direction,
        config,
    );
    let intake_attempt_nodes = unblocked_intake_requests
        .iter()
        .filter(|request| request.requested_amount > 0)
        .map(|request| request.node_index)
        .collect::<HashSet<_>>();
    let blocked_intake_direction_nodes = buffered_outlet_nodes
        .intersection(&intake_attempt_nodes)
        .copied()
        .collect::<HashSet<_>>();
    let stopped_segment_edges =
        segment_edges_for_blocked_intake_nodes(&runtime, &blocked_intake_direction_nodes);
    let vent_intake_requests = build_vent_requests(
        &runtime,
        &vent_contexts,
        &intake_vent_offers,
        &intake_outgoing_request_counts,
        &forward_demand_intake_nodes,
        &buffered_outlet_nodes,
        &intake_allowed_direction,
        config,
    )
    .into_iter()
    .filter(|request| matches!(request.direction, VentRequestDirection::WorldToPipe))
    .collect::<Vec<_>>();

    let mut world_changed = false;
    for request in &vent_intake_requests {
        if request.requested_amount == 0 {
            continue;
        }
        let per_hop_capacity = config
            .max_pipe_hop_particles_per_step
            .saturating_mul(runtime.nodes[request.node_index].neighbors.len().max(1) as u32)
            .max(1);
        let already_in_node: u32 = next_pipe_species[request.node_index].iter().copied().sum();
        let room_left = per_hop_capacity.saturating_sub(already_in_node);
        if room_left == 0 {
            continue;
        }
        let removed = remove_particles_from_world_cells(
            gas,
            &request.world_cells,
            request.requested_amount.min(room_left),
            world,
        );
        if removed.iter().all(|count| *count == 0) {
            continue;
        }
        add_species_counts(&mut next_pipe_species[request.node_index], &removed);
        world_changed = true;
    }

    for entry in vent_buffers.iter_mut() {
        world_changed |=
            add_particles_to_world_cells(gas, &entry.world_cells, &entry.species_counts, world);
    }

    for node_id in 0..pipe_gas.node_count() {
        pipe_changed |= next_pipe_species[node_id] != species_before_vent[node_id];
        pipe_gas.set_species_counts_exact(node_id, &next_pipe_species[node_id]);
    }

    if world_changed {
        gas.recompute_total_density_buffer(world);
    }

    visual_state.transfers.clear();
    let planning_totals = totals_from_species(&next_pipe_species);
    let segment_locked_candidates = lock_candidates_to_allowed_direction(
        &runtime,
        &planning_totals,
        &intake_allowed_direction,
        config,
    );
    let accepted_pipe_requests =
        build_pipe_transfer_requests(&segment_locked_candidates, &planning_totals, config);

    let mut planned_species = next_pipe_species.clone();
    for request in &accepted_pipe_requests {
        if stopped_segment_edges.contains(&UndirectedPipeEdge::new(
            request.source_index,
            request.target_index,
        )) {
            continue;
        }
        if request.requested_amount == 0 {
            continue;
        }
        let moved = remove_species_proportional_counts(
            &mut planned_species[request.source_index],
            request.requested_amount,
        );
        if moved.iter().all(|count| *count == 0) {
            continue;
        }
        visual_state.transfers.push(PipeTransferRecord {
            from: runtime.nodes[request.source_index].visual_cell,
            from_kind: pipe_gas.keys[request.source_index].kind,
            to: runtime.nodes[request.target_index].visual_cell,
            to_kind: pipe_gas.keys[request.target_index].kind,
            total_amount: moved.iter().copied().sum(),
            gas_counts: moved,
            visual_path: transfer_visual_path(
                pipe_gas.keys[request.source_index],
                pipe_gas.keys[request.target_index],
                structures,
            ),
        });
    }

    pipe_changed || world_changed
}

pub(super) fn build_pipe_debug_hud_lines_for_cell(
    cell: UVec2,
    structures: &PlacedStructureMap,
    pipe_gas: &PipeGasField,
    gas: &GasField,
    config: &PipeSimulationConfig,
    flow_state: &PipeFlowVisualState,
) -> Vec<String> {
    let mut shadow_pipe = pipe_gas.clone();
    shadow_pipe.sync_to_structures(structures);
    let runtime = PipeRuntime::from_structures(structures, &shadow_pipe);
    if runtime.nodes.is_empty() {
        return Vec::new();
    }

    let mut node_ids = node_ids_for_cell(structures, &shadow_pipe, cell.x, cell.y);
    if node_ids.is_empty() {
        return Vec::new();
    }
    node_ids.sort_unstable();

    let species_before = (0..shadow_pipe.node_count())
        .map(|node_id| shadow_pipe.node_species_counts(node_id))
        .collect::<Vec<_>>();
    let mut next_pipe_species = species_before.clone();

    let vent_contexts = collect_vent_contexts(&runtime, gas, config);
    let vent_context_by_node = vent_contexts
        .iter()
        .map(|context| (context.node_index, context.clone()))
        .collect::<HashMap<_, _>>();
    let vent_buffers = drain_vent_pipe_nodes_to_buffer(&vent_contexts, &mut next_pipe_species);
    let buffered_outlet_nodes = vent_buffers
        .iter()
        .map(|entry| entry.node_index)
        .collect::<HashSet<_>>();
    let totals_before_intake = totals_from_species(&next_pipe_species);
    let vent_world_pressures = build_vent_world_pressure_lut(runtime.nodes.len(), &vent_contexts);
    let vent_offers = build_vent_pressure_offer_lut(&runtime, &vent_world_pressures, config);
    let (edge_demand_pa, outgoing_request_counts) =
        build_edge_offer_demands(&runtime, &vent_offers, config);
    let allowed_direction =
        segment_allowed_direction_map(&runtime, &edge_demand_pa, &vent_world_pressures, config);
    let forward_demand_intake_nodes =
        vent_nodes_with_forward_direction(&runtime, &allowed_direction);
    let unblocked_intake_requests = build_vent_requests(
        &runtime,
        &vent_contexts,
        &vent_offers,
        &outgoing_request_counts,
        &forward_demand_intake_nodes,
        &HashSet::new(),
        &allowed_direction,
        config,
    );
    let blocked_intake_requests = build_vent_requests(
        &runtime,
        &vent_contexts,
        &vent_offers,
        &outgoing_request_counts,
        &forward_demand_intake_nodes,
        &buffered_outlet_nodes,
        &allowed_direction,
        config,
    );
    let intake_attempt_nodes = unblocked_intake_requests
        .iter()
        .filter(|request| request.requested_amount > 0)
        .map(|request| request.node_index)
        .collect::<HashSet<_>>();
    let blocked_intake_direction_nodes = buffered_outlet_nodes
        .intersection(&intake_attempt_nodes)
        .copied()
        .collect::<HashSet<_>>();

    let unblocked_requests_by_node = unblocked_intake_requests
        .iter()
        .map(|request| (request.node_index, request.requested_amount))
        .collect::<HashMap<_, _>>();
    let blocked_requests_by_node = blocked_intake_requests
        .iter()
        .map(|request| (request.node_index, request.requested_amount))
        .collect::<HashMap<_, _>>();

    let mut lines = Vec::new();
    lines.push("[DEBUG] Pipe/Vent".to_string());
    lines.push(format!(
        "ph i={} t={} p={:.3} h={}",
        flow_state
            .interval_ticks
            .max(config.pipe_step_interval_ticks.max(1)),
        flow_state.tick_in_interval,
        flow_state.flow_progress(),
        if flow_state.hop_started_this_tick {
            1
        } else {
            0
        }
    ));

    let mut edge_outlet_cache = HashMap::new();
    let segments = collect_pipe_segments(&runtime);
    let mut forward_outlet_cache = HashMap::new();
    for node_id in node_ids {
        let node = &runtime.nodes[node_id];
        let key = shadow_pipe.keys[node_id];
        let pre_total = species_before[node_id].iter().copied().sum::<u32>();
        let eval_total = totals_before_intake.get(node_id).copied().unwrap_or(0);
        lines.push(format!(
            "n{} {:?} ({}, {})",
            node_id, key.kind, node.visual_cell.x, node.visual_cell.y
        ));
        lines.push(format!(
            "ppre={:.0} peval={:.0} nb={}",
            pipe_pressure_pa(config, pre_total),
            pipe_pressure_pa(config, eval_total),
            node.neighbors.len()
        ));
        lines.push(format!("m pre={} eval={}", pre_total, eval_total,));
        if let Some(segment) = segments.iter().find(|segment| segment.contains(&node_id)) {
            if let (Some(start), Some(end)) = (segment.first().copied(), segment.last().copied()) {
                let start_world = vent_world_pressures.get(start).copied().flatten();
                let end_world = vent_world_pressures.get(end).copied().flatten();
                let direction = segment_direction_label(segment, &allowed_direction);
                lines.push(format!(
                    "seg n{}->n{} d={} Ps={:.0} Pe={:.0}",
                    start,
                    end,
                    direction,
                    start_world.unwrap_or(-1.0),
                    end_world.unwrap_or(-1.0)
                ));
            }
        }

        let mut neighbors = node.neighbors.clone();
        neighbors.sort_unstable();
        for neighbor in neighbors {
            let target = runtime.nodes[neighbor].visual_cell;
            let direction_allowed = allowed_direction
                .get(&(node_id, neighbor))
                .copied()
                .unwrap_or(false);
            let has_outlet =
                edge_has_downstream_outlet(&runtime, node_id, neighbor, &mut edge_outlet_cache);
            let demand = edge_demand_pa.get(&(node_id, neighbor)).copied();
            let demand_reverse = edge_demand_pa.get(&(neighbor, node_id)).copied();
            lines.push(format!(
                "e->n{} a={} o={} df={} dr={}",
                neighbor,
                if direction_allowed { 1 } else { 0 },
                if has_outlet { 1 } else { 0 },
                demand.unwrap_or(0.0).round() as i64,
                demand_reverse.unwrap_or(0.0).round() as i64
            ));
            let _ = target;
        }
        append_live_transfer_lines_for_node(
            &mut lines,
            node.visual_cell,
            key.kind,
            &flow_state.previous_transfers,
            &flow_state.transfers,
        );

        if node.vent_cell.is_none() {
            lines.push("vent: none".to_string());
            continue;
        }
        let Some(context) = vent_context_by_node.get(&node_id) else {
            lines.push("vent: missing context".to_string());
            continue;
        };
        let world_particles = context
            .world_cells
            .iter()
            .map(|world_cell| gas.total_amount_rounded(world_cell.x, world_cell.y))
            .sum::<u32>();
        let world_pressure = context.world_pressure;
        let offer = vent_offers.get(node_id).copied().unwrap_or(0.0);
        let has_forward_outlet = count_forward_outlet_edges(
            &runtime,
            node_id,
            &allowed_direction,
            &mut forward_outlet_cache,
        ) > 0;
        let in_forward_demand = forward_demand_intake_nodes.contains(&node_id);
        let buffered = buffered_outlet_nodes.contains(&node_id);
        let blocked_direction = blocked_intake_direction_nodes.contains(&node_id);
        let outgoing_request_count = outgoing_request_counts
            .get(node_id)
            .copied()
            .unwrap_or(0)
            .max(1);
        let offer_intake = if offer < 0.0 {
            offer_based_intake_particles(config, -offer, outgoing_request_count)
        } else {
            0
        };
        let intake_unblocked = unblocked_requests_by_node
            .get(&node_id)
            .copied()
            .unwrap_or(0);
        let intake_after_buffer = blocked_requests_by_node.get(&node_id).copied().unwrap_or(0);
        let per_hop_capacity = config
            .max_pipe_hop_particles_per_step
            .saturating_mul(node.neighbors.len().max(1) as u32)
            .max(1);
        let already_in_node = next_pipe_species
            .get(node_id)
            .map(|species| species.iter().copied().sum())
            .unwrap_or(0);
        let room_left = per_hop_capacity.saturating_sub(already_in_node);
        let can_intake = in_forward_demand
            && has_forward_outlet
            && !buffered
            && intake_unblocked > 0
            && room_left > 0
            && world_particles > 0;
        lines.push(format!(
            "v wp={:.0} wt={} of={:.0} n={} o={} fd={} b={} bd={}",
            world_pressure,
            world_particles,
            offer,
            outgoing_request_count,
            if has_forward_outlet { 1 } else { 0 },
            if in_forward_demand { 1 } else { 0 },
            if buffered { 1 } else { 0 },
            if blocked_direction { 1 } else { 0 }
        ));
        let mut reasons = Vec::new();
        if !in_forward_demand {
            reasons.push("fd");
        }
        if !has_forward_outlet {
            reasons.push("out");
        }
        if buffered {
            reasons.push("buf");
        }
        if intake_unblocked == 0 {
            reasons.push("rq0");
        }
        if room_left == 0 {
            reasons.push("full");
        }
        if world_particles == 0 {
            reasons.push("wg0");
        }
        lines.push(format!(
            "i rq={} ru={} ra={} rl={} ci={} g={}",
            offer_intake,
            intake_unblocked,
            intake_after_buffer,
            room_left,
            if can_intake { 1 } else { 0 },
            if reasons.is_empty() {
                "ok".to_string()
            } else {
                reasons.join("|")
            }
        ));
    }

    lines
}

fn append_live_transfer_lines_for_node(
    lines: &mut Vec<String>,
    node_cell: UVec2,
    node_kind: PipeContainerKind,
    previous_transfers: &[PipeTransferRecord],
    current_transfers: &[PipeTransferRecord],
) {
    let committed =
        summarize_transfer_directions_for_node(previous_transfers, node_cell, node_kind);
    let planned = summarize_transfer_directions_for_node(current_transfers, node_cell, node_kind);
    lines.push(format!("tr prev:{} plan:{}", committed, planned));
}

fn summarize_transfer_directions_for_node(
    transfers: &[PipeTransferRecord],
    node_cell: UVec2,
    node_kind: PipeContainerKind,
) -> String {
    let mut outgoing = 0u64;
    let mut incoming = 0u64;
    let mut outgoing_dirs = Vec::new();
    let mut incoming_dirs = Vec::new();
    for transfer in transfers {
        if transfer.total_amount == 0 {
            continue;
        }
        if transfer.from_kind == node_kind && transfer.from == node_cell {
            outgoing = outgoing.saturating_add(u64::from(transfer.total_amount));
            outgoing_dirs.push(direction_label(transfer.from, transfer.to));
        }
        if transfer.to_kind == node_kind && transfer.to == node_cell {
            incoming = incoming.saturating_add(u64::from(transfer.total_amount));
            incoming_dirs.push(direction_label(transfer.to, transfer.from));
        }
    }
    if outgoing == 0 && incoming == 0 {
        return "-".to_string();
    }
    outgoing_dirs.sort();
    outgoing_dirs.dedup();
    incoming_dirs.sort();
    incoming_dirs.dedup();
    format!(
        "i{}:{} o{}:{}",
        incoming,
        if incoming_dirs.is_empty() {
            "-".to_string()
        } else {
            incoming_dirs.join(",")
        },
        outgoing,
        if outgoing_dirs.is_empty() {
            "-".to_string()
        } else {
            outgoing_dirs.join(",")
        }
    )
}

fn direction_label(from: UVec2, to: UVec2) -> String {
    let dx = to.x as i64 - from.x as i64;
    let dy = to.y as i64 - from.y as i64;
    match (dx.signum(), dy.signum()) {
        (1, 0) => "E".to_string(),
        (-1, 0) => "W".to_string(),
        (0, 1) => "S".to_string(),
        (0, -1) => "N".to_string(),
        _ => format!("dx{}dy{}", dx, dy),
    }
}

fn segment_direction_label(
    segment: &[usize],
    allowed_direction: &HashMap<(usize, usize), bool>,
) -> &'static str {
    if segment.len() < 2 {
        return "n/a";
    }
    let start = segment[0];
    let next = segment[1];
    match allowed_direction.get(&(start, next)).copied() {
        Some(true) => "start->end",
        Some(false) => "end->start",
        None => "unlocked",
    }
}

fn totals_from_species(species: &[Vec<u32>]) -> Vec<u32> {
    species
        .iter()
        .map(|counts| counts.iter().copied().sum())
        .collect()
}

fn commit_planned_pipe_transfers(
    runtime: &PipeRuntime,
    pipe_gas: &mut PipeGasField,
    planned_transfers: &[PipeTransferRecord],
) -> bool {
    if planned_transfers.is_empty() {
        return false;
    }

    let mut node_by_visual = HashMap::new();
    for (node_id, node) in runtime.nodes.iter().enumerate() {
        node_by_visual.insert((pipe_gas.keys[node_id].kind, node.visual_cell), node_id);
    }

    let mut species = (0..pipe_gas.node_count())
        .map(|node_id| pipe_gas.node_species_counts(node_id))
        .collect::<Vec<_>>();
    let before = species.clone();

    for transfer in planned_transfers {
        if transfer.total_amount == 0 {
            continue;
        }
        let Some(source_index) = node_by_visual
            .get(&(transfer.from_kind, transfer.from))
            .copied()
        else {
            continue;
        };
        let Some(target_index) = node_by_visual
            .get(&(transfer.to_kind, transfer.to))
            .copied()
        else {
            continue;
        };

        let moved = remove_species_exact_counts(&mut species[source_index], &transfer.gas_counts);
        if moved.iter().all(|count| *count == 0) {
            continue;
        }
        add_species_counts(&mut species[target_index], &moved);
    }

    let mut changed = false;
    for node_id in 0..pipe_gas.node_count() {
        changed |= species[node_id] != before[node_id];
        pipe_gas.set_species_counts_exact(node_id, &species[node_id]);
    }
    changed
}

fn collect_vent_contexts(
    runtime: &PipeRuntime,
    gas: &GasField,
    config: &PipeSimulationConfig,
) -> Vec<VentContext> {
    let mut contexts = Vec::new();
    for (node_index, node) in runtime.nodes.iter().enumerate() {
        let Some(cell) = node.vent_cell else {
            continue;
        };
        let world_cells = collect_vent_world_cells(cell);
        let (world_pressure, _) = sampled_world_reservoir(gas, config, &world_cells);
        contexts.push(VentContext {
            node_index,
            world_cells,
            world_pressure,
        });
    }
    contexts
}

fn drain_vent_pipe_nodes_to_buffer(
    vent_contexts: &[VentContext],
    next_pipe_species: &mut [Vec<u32>],
) -> Vec<VentBufferEntry> {
    let mut buffers = Vec::new();
    for context in vent_contexts {
        let Some(node_species) = next_pipe_species.get_mut(context.node_index) else {
            continue;
        };
        if node_species.iter().all(|count| *count == 0) {
            continue;
        }
        let drained = node_species.clone();
        node_species.fill(0);
        buffers.push(VentBufferEntry {
            node_index: context.node_index,
            world_cells: context.world_cells.clone(),
            species_counts: drained,
        });
    }
    buffers
}

fn build_vent_world_pressure_lut(
    node_count: usize,
    vent_contexts: &[VentContext],
) -> Vec<Option<f32>> {
    let mut lut = vec![None; node_count];
    for context in vent_contexts {
        if context.node_index >= lut.len() {
            continue;
        }
        lut[context.node_index] = Some(context.world_pressure.max(0.0));
    }
    lut
}

fn build_vent_pressure_offer_lut(
    runtime: &PipeRuntime,
    vent_world_pressures: &[Option<f32>],
    config: &PipeSimulationConfig,
) -> Vec<f32> {
    let mut offers = vec![0.0f32; runtime.nodes.len()];
    let epsilon = config.pressure_epsilon_pa.max(0.0);
    for component in collect_runtime_components(runtime) {
        let mut vents = Vec::<(usize, f32)>::new();
        for node_index in component {
            if runtime.nodes[node_index].vent_cell.is_none() {
                continue;
            }
            let pressure = vent_world_pressures
                .get(node_index)
                .and_then(|value| *value)
                .unwrap_or(0.0)
                .max(0.0);
            vents.push((node_index, pressure));
        }
        if vents.len() < 2 {
            continue;
        }
        let pressure_sum = vents.iter().map(|(_, pressure)| *pressure).sum::<f32>();
        let vent_count = vents.len() as f32;
        for (node_index, pressure) in vents {
            let offer = pressure_sum - pressure * vent_count;
            offers[node_index] = if offer.abs() <= epsilon { 0.0 } else { offer };
        }
    }
    offers
}

fn build_edge_offer_demands(
    runtime: &PipeRuntime,
    vent_offers: &[f32],
    config: &PipeSimulationConfig,
) -> (HashMap<(usize, usize), f32>, Vec<u32>) {
    let mut demand = HashMap::<(usize, usize), f32>::new();
    let mut source_outgoing_request_counts = vec![0u32; runtime.nodes.len()];
    let epsilon = config.pressure_epsilon_pa.max(0.0);
    for component in collect_runtime_components(runtime) {
        let mut sources = Vec::<(usize, f32)>::new();
        let mut sinks = Vec::<(usize, f32)>::new();
        for node_index in component {
            if runtime.nodes[node_index].vent_cell.is_none() {
                continue;
            }
            let offer = vent_offers.get(node_index).copied().unwrap_or(0.0);
            if offer < -epsilon {
                sources.push((node_index, -offer));
            } else if offer > epsilon {
                sinks.push((node_index, offer));
            }
        }
        if sources.is_empty() || sinks.is_empty() {
            continue;
        }
        let total_sink_demand = sinks
            .iter()
            .map(|(_, sink)| *sink)
            .sum::<f32>()
            .max(epsilon);
        for (source_node, source_supply) in sources {
            let mut outgoing_request_neighbors = HashSet::new();
            for (sink_node, sink_demand) in &sinks {
                let pair_demand = source_supply * (*sink_demand / total_sink_demand);
                if pair_demand <= epsilon {
                    continue;
                }
                let Some(path) = shortest_path_nodes(runtime, source_node, *sink_node) else {
                    continue;
                };
                if path.len() < 2 {
                    continue;
                }
                outgoing_request_neighbors.insert(path[1]);
                for edge in path.windows(2) {
                    let key = (edge[0], edge[1]);
                    *demand.entry(key).or_insert(0.0) += pair_demand;
                }
            }
            if source_node < source_outgoing_request_counts.len() {
                source_outgoing_request_counts[source_node] = source_outgoing_request_counts
                    [source_node]
                    .max(outgoing_request_neighbors.len() as u32);
            }
        }
    }
    (demand, source_outgoing_request_counts)
}

fn collect_runtime_components(runtime: &PipeRuntime) -> Vec<Vec<usize>> {
    let mut components = Vec::new();
    let mut visited = vec![false; runtime.nodes.len()];
    for start in 0..runtime.nodes.len() {
        if visited[start] {
            continue;
        }
        let mut queue = VecDeque::new();
        let mut component = Vec::new();
        visited[start] = true;
        queue.push_back(start);
        while let Some(node_index) = queue.pop_front() {
            component.push(node_index);
            let mut neighbors = runtime.nodes[node_index].neighbors.clone();
            neighbors.sort_unstable();
            for neighbor in neighbors {
                if visited[neighbor] {
                    continue;
                }
                visited[neighbor] = true;
                queue.push_back(neighbor);
            }
        }
        components.push(component);
    }
    components
}

fn shortest_path_nodes(runtime: &PipeRuntime, start: usize, goal: usize) -> Option<Vec<usize>> {
    if start == goal {
        return Some(vec![start]);
    }
    if start >= runtime.nodes.len() || goal >= runtime.nodes.len() {
        return None;
    }
    let mut parents = vec![usize::MAX; runtime.nodes.len()];
    let mut visited = vec![false; runtime.nodes.len()];
    let mut queue = VecDeque::new();
    visited[start] = true;
    queue.push_back(start);
    while let Some(node_index) = queue.pop_front() {
        let mut neighbors = runtime.nodes[node_index].neighbors.clone();
        neighbors.sort_unstable();
        for neighbor in neighbors {
            if visited[neighbor] {
                continue;
            }
            visited[neighbor] = true;
            parents[neighbor] = node_index;
            if neighbor == goal {
                let mut path = vec![goal];
                let mut cursor = goal;
                while cursor != start {
                    let parent = parents[cursor];
                    if parent == usize::MAX {
                        return None;
                    }
                    path.push(parent);
                    cursor = parent;
                }
                path.reverse();
                return Some(path);
            }
            queue.push_back(neighbor);
        }
    }
    None
}

fn nearest_vent_distance_lut(runtime: &PipeRuntime) -> Vec<Option<u32>> {
    let mut distance: Vec<Option<u32>> = vec![None; runtime.nodes.len()];
    let mut queue = VecDeque::new();
    for (node_index, node) in runtime.nodes.iter().enumerate() {
        if node.vent_cell.is_none() {
            continue;
        }
        distance[node_index] = Some(0);
        queue.push_back(node_index);
    }
    while let Some(node_index) = queue.pop_front() {
        let Some(current_distance) = distance[node_index] else {
            continue;
        };
        let mut neighbors = runtime.nodes[node_index].neighbors.clone();
        neighbors.sort_unstable();
        for neighbor in neighbors {
            if distance[neighbor].is_some() {
                continue;
            }
            distance[neighbor] = Some(current_distance.saturating_add(1));
            queue.push_back(neighbor);
        }
    }
    distance
}

fn edge_has_downstream_outlet(
    runtime: &PipeRuntime,
    source_index: usize,
    target_index: usize,
    cache: &mut HashMap<(usize, usize), bool>,
) -> bool {
    if let Some(cached) = cache.get(&(source_index, target_index)).copied() {
        return cached;
    }

    let mut visited = vec![false; runtime.nodes.len()];
    let mut queue = VecDeque::new();
    visited[target_index] = true;
    queue.push_back(target_index);

    let mut found_outlet = false;
    while let Some(node_index) = queue.pop_front() {
        if runtime.nodes[node_index].vent_cell.is_some() {
            found_outlet = true;
            break;
        }
        for neighbor in runtime.nodes[node_index].neighbors.iter().copied() {
            if node_index == target_index && neighbor == source_index {
                continue;
            }
            if visited[neighbor] {
                continue;
            }
            visited[neighbor] = true;
            queue.push_back(neighbor);
        }
    }

    cache.insert((source_index, target_index), found_outlet);
    found_outlet
}

fn count_forward_outlet_edges(
    runtime: &PipeRuntime,
    node_index: usize,
    allowed_direction: &HashMap<(usize, usize), bool>,
    outlet_cache: &mut HashMap<(usize, usize), bool>,
) -> usize {
    runtime.nodes[node_index]
        .neighbors
        .iter()
        .copied()
        .filter(|neighbor| {
            allowed_direction
                .get(&(node_index, *neighbor))
                .copied()
                .unwrap_or(false)
                && edge_has_downstream_outlet(runtime, node_index, *neighbor, outlet_cache)
        })
        .count()
}

fn vent_nodes_with_forward_direction(
    runtime: &PipeRuntime,
    allowed_direction: &HashMap<(usize, usize), bool>,
) -> HashSet<usize> {
    let mut intake_nodes = HashSet::new();
    let mut outlet_cache = HashMap::new();
    for (node_index, node) in runtime.nodes.iter().enumerate() {
        if node.vent_cell.is_none() {
            continue;
        }
        if count_forward_outlet_edges(runtime, node_index, allowed_direction, &mut outlet_cache) > 0
        {
            intake_nodes.insert(node_index);
        }
    }
    intake_nodes
}

fn build_pipe_transfer_requests(
    candidates: &[DirectedPipeCandidate],
    starting_totals: &[u32],
    config: &PipeSimulationConfig,
) -> Vec<DirectedPipeRequest> {
    let mut by_source: HashMap<usize, Vec<(usize, u32)>> = HashMap::new();
    for candidate in candidates {
        by_source
            .entry(candidate.source_index)
            .or_default()
            .push((candidate.target_index, candidate.demand_particles));
    }

    let mut accepted = Vec::new();
    for (source_index, mut outgoing) in by_source {
        if outgoing.is_empty() {
            continue;
        }
        outgoing.sort_by_key(|(target_index, _)| *target_index);

        let branch_residual = if outgoing.len() > 1 {
            config
                .min_pipe_branch_residual_particles
                .min(starting_totals[source_index])
        } else {
            0
        };
        let mut available = starting_totals[source_index].saturating_sub(branch_residual);
        let max_total_hop = config
            .max_pipe_hop_particles_per_step
            .saturating_mul(outgoing.len() as u32)
            .max(1);
        available = available.min(max_total_hop);
        if available == 0 {
            continue;
        }

        let requests = outgoing
            .iter()
            .map(|(_, demand)| *demand)
            .collect::<Vec<_>>();
        let requested_total = requests.iter().copied().sum::<u32>();
        let accepted_total = available.min(requested_total);
        if accepted_total == 0 {
            continue;
        }

        let mut split = split_bounded_integer_requests(accepted_total, &requests);
        if accepted_total > 0
            && outgoing.len() == 1
            && requested_total > 0
            && split.first().copied().unwrap_or(0) == 0
        {
            split[0] = 1;
        }
        for ((target_index, _), accepted_amount) in outgoing.into_iter().zip(split) {
            if accepted_amount == 0 {
                continue;
            }
            accepted.push(DirectedPipeRequest {
                source_index,
                target_index,
                requested_amount: accepted_amount,
            });
        }
    }

    accepted
}

fn segment_allowed_direction_map(
    runtime: &PipeRuntime,
    edge_demand_pa: &HashMap<(usize, usize), f32>,
    vent_world_pressures: &[Option<f32>],
    config: &PipeSimulationConfig,
) -> HashMap<(usize, usize), bool> {
    let mut allowed_direction = HashMap::new();
    let epsilon = config.pressure_epsilon_pa.max(0.0);
    let vent_distance = nearest_vent_distance_lut(runtime);
    for segment in collect_pipe_segments(runtime) {
        let Some(start) = segment.first().copied() else {
            continue;
        };
        let Some(end) = segment.last().copied() else {
            continue;
        };
        let mut forward_demand = 0.0f32;
        let mut backward_demand = 0.0f32;
        for edge in segment.windows(2) {
            let source = edge[0];
            let target = edge[1];
            forward_demand += edge_demand_pa
                .get(&(source, target))
                .copied()
                .unwrap_or(0.0);
            backward_demand += edge_demand_pa
                .get(&(target, source))
                .copied()
                .unwrap_or(0.0);
        }
        let use_forward = if forward_demand > backward_demand + epsilon {
            Some(true)
        } else if backward_demand > forward_demand + epsilon {
            Some(false)
        } else {
            let start_world_pressure = runtime.nodes[start]
                .vent_cell
                .and_then(|_| vent_world_pressures.get(start).copied().flatten());
            let end_world_pressure = runtime.nodes[end]
                .vent_cell
                .and_then(|_| vent_world_pressures.get(end).copied().flatten());
            match (start_world_pressure, end_world_pressure) {
                (Some(start_pressure), Some(end_pressure))
                    if start_pressure - end_pressure > epsilon =>
                {
                    Some(true)
                }
                (Some(start_pressure), Some(end_pressure))
                    if end_pressure - start_pressure > epsilon =>
                {
                    Some(false)
                }
                _ => {
                    let start_distance = vent_distance.get(start).and_then(|value| *value);
                    let end_distance = vent_distance.get(end).and_then(|value| *value);
                    match (start_distance, end_distance) {
                        (Some(start_steps), Some(end_steps)) if start_steps > end_steps => {
                            Some(true)
                        }
                        (Some(start_steps), Some(end_steps)) if end_steps > start_steps => {
                            Some(false)
                        }
                        _ => None,
                    }
                }
            }
        };
        let Some(use_forward) = use_forward else {
            continue;
        };

        for edge in segment.windows(2) {
            let source = edge[0];
            let target = edge[1];
            if use_forward {
                allowed_direction.insert((source, target), true);
                allowed_direction.insert((target, source), false);
            } else {
                allowed_direction.insert((source, target), false);
                allowed_direction.insert((target, source), true);
            }
        }
    }

    allowed_direction
}

fn lock_candidates_to_allowed_direction(
    runtime: &PipeRuntime,
    node_totals: &[u32],
    allowed_direction: &HashMap<(usize, usize), bool>,
    config: &PipeSimulationConfig,
) -> Vec<DirectedPipeCandidate> {
    let mut expanded = Vec::new();
    for segment in collect_pipe_segments(runtime) {
        for edge in segment.windows(2) {
            let a = edge[0];
            let b = edge[1];
            let forward_allowed = allowed_direction.get(&(a, b)).copied().unwrap_or(false);
            let backward_allowed = allowed_direction.get(&(b, a)).copied().unwrap_or(false);
            let (source, target) = if forward_allowed {
                (a, b)
            } else if backward_allowed {
                (b, a)
            } else {
                continue;
            };
            let available = node_totals.get(source).copied().unwrap_or(0);
            if available == 0 {
                continue;
            }
            expanded.push(DirectedPipeCandidate {
                source_index: source,
                target_index: target,
                demand_particles: available.min(config.max_pipe_hop_particles_per_step.max(1)),
            });
        }
    }

    expanded.sort_by_key(|candidate| (candidate.source_index, candidate.target_index));
    expanded.dedup_by(|left, right| {
        left.source_index == right.source_index && left.target_index == right.target_index
    });
    expanded
}

fn collect_pipe_segments(runtime: &PipeRuntime) -> Vec<Vec<usize>> {
    let mut visited_edges = HashSet::new();
    let mut segments = Vec::new();
    let node_count = runtime.nodes.len();

    for start in 0..node_count {
        if !is_segment_boundary_node(runtime, start) {
            continue;
        }
        for neighbor in runtime.nodes[start].neighbors.iter().copied() {
            let edge = UndirectedPipeEdge::new(start, neighbor);
            if visited_edges.contains(&edge) {
                continue;
            }
            let segment = walk_segment(runtime, start, neighbor, &mut visited_edges);
            if segment.len() >= 2 {
                segments.push(segment);
            }
        }
    }

    for start in 0..node_count {
        for neighbor in runtime.nodes[start].neighbors.iter().copied() {
            let edge = UndirectedPipeEdge::new(start, neighbor);
            if visited_edges.contains(&edge) {
                continue;
            }
            let segment = walk_segment(runtime, start, neighbor, &mut visited_edges);
            if segment.len() >= 2 {
                segments.push(segment);
            }
        }
    }

    segments
}

fn segment_edges_for_blocked_intake_nodes(
    runtime: &PipeRuntime,
    blocked_nodes: &HashSet<usize>,
) -> HashSet<UndirectedPipeEdge> {
    if blocked_nodes.is_empty() {
        return HashSet::new();
    }
    let mut blocked_edges = HashSet::new();
    for segment in collect_pipe_segments(runtime) {
        if !segment.iter().any(|node| blocked_nodes.contains(node)) {
            continue;
        }
        for edge in segment.windows(2) {
            blocked_edges.insert(UndirectedPipeEdge::new(edge[0], edge[1]));
        }
    }
    blocked_edges
}

fn walk_segment(
    runtime: &PipeRuntime,
    start: usize,
    first_neighbor: usize,
    visited_edges: &mut HashSet<UndirectedPipeEdge>,
) -> Vec<usize> {
    let mut segment = vec![start, first_neighbor];
    visited_edges.insert(UndirectedPipeEdge::new(start, first_neighbor));
    let mut previous = start;
    let mut current = first_neighbor;

    for _ in 0..runtime.nodes.len().saturating_add(1) {
        if is_segment_boundary_node(runtime, current) {
            break;
        }
        let Some(next) = runtime.nodes[current]
            .neighbors
            .iter()
            .copied()
            .find(|neighbor| *neighbor != previous)
        else {
            break;
        };
        let edge = UndirectedPipeEdge::new(current, next);
        if visited_edges.contains(&edge) {
            break;
        }
        visited_edges.insert(edge);
        segment.push(next);
        previous = current;
        current = next;
    }

    segment
}

fn is_segment_boundary_node(runtime: &PipeRuntime, node_index: usize) -> bool {
    runtime.nodes[node_index].vent_cell.is_some() || runtime.nodes[node_index].neighbors.len() != 2
}

fn build_vent_requests(
    runtime: &PipeRuntime,
    vent_contexts: &[VentContext],
    vent_offers: &[f32],
    outgoing_request_counts: &[u32],
    forward_demand_nodes: &HashSet<usize>,
    blocked_intake_nodes: &HashSet<usize>,
    allowed_direction: &HashMap<(usize, usize), bool>,
    config: &PipeSimulationConfig,
) -> Vec<VentRequest> {
    let mut requests = Vec::new();
    let mut outlet_cache = HashMap::new();
    let epsilon = config.pressure_epsilon_pa.max(0.0);

    for context in vent_contexts {
        let node_index = context.node_index;
        if blocked_intake_nodes.contains(&node_index) || !forward_demand_nodes.contains(&node_index)
        {
            continue;
        }
        let forward_outlet_edges =
            count_forward_outlet_edges(runtime, node_index, allowed_direction, &mut outlet_cache);
        if forward_outlet_edges == 0 {
            continue;
        }
        let offer = vent_offers.get(node_index).copied().unwrap_or(0.0);
        if offer >= -epsilon {
            continue;
        }
        let split_paths = outgoing_request_counts
            .get(node_index)
            .copied()
            .unwrap_or(0)
            .max(1);
        let intake_request = offer_based_intake_particles(config, -offer, split_paths);
        if intake_request > 0 {
            requests.push(VentRequest {
                node_index,
                requested_amount: intake_request,
                direction: VentRequestDirection::WorldToPipe,
                world_cells: context.world_cells.clone(),
            });
        }
    }

    requests
}

fn offer_based_intake_particles(
    config: &PipeSimulationConfig,
    offer_pa: f32,
    path_count: u32,
) -> u32 {
    if offer_pa <= config.pressure_epsilon_pa.max(0.0) {
        return 0;
    }
    let split_paths = path_count.max(1) as f32;
    let reduced_delta_pa = offer_pa / split_paths;
    let reduced_delta_particles = reduced_delta_pa / config.cell_particle_pressure_pa.max(1e-6);
    let requested_linear = reduced_delta_particles / config.cell_volume_ratio.max(1.0);
    let max_flux = config
        .max_vent_flux_particles_per_tick
        .max(0.0)
        .min(200_000.0);
    if max_flux <= 0.0 {
        return 0;
    }
    let mut requested = requested_linear.max(0.0).min(max_flux).round() as u32;
    if reduced_delta_particles >= 5.0 && requested == 0 {
        requested = 1;
    }
    requested
}

#[cfg(test)]
pub(super) fn offer_based_intake_particles_for_test(
    config: &PipeSimulationConfig,
    offer_pa: f32,
    path_count: u32,
) -> u32 {
    offer_based_intake_particles(config, offer_pa, path_count)
}

fn add_species_counts(target: &mut [u32], offered: &[u32]) {
    for gas_index in 0..target.len() {
        target[gas_index] =
            target[gas_index].saturating_add(offered.get(gas_index).copied().unwrap_or(0));
    }
}

fn remove_species_exact_counts(source: &mut [u32], requested: &[u32]) -> Vec<u32> {
    let mut removed = vec![0u32; source.len()];
    for gas_index in 0..source.len() {
        let request = requested.get(gas_index).copied().unwrap_or(0);
        let accepted = source[gas_index].min(request);
        source[gas_index] = source[gas_index].saturating_sub(accepted);
        removed[gas_index] = accepted;
    }
    removed
}

fn collect_vent_world_cells(center: UVec2) -> Vec<UVec2> {
    vec![center]
}

fn sampled_world_reservoir(
    gas: &GasField,
    config: &PipeSimulationConfig,
    world_cells: &[UVec2],
) -> (f32, u32) {
    let total_particles: u32 = world_cells
        .iter()
        .map(|cell| gas.total_amount_rounded(cell.x, cell.y))
        .sum();
    let average_particles = if world_cells.is_empty() {
        0
    } else {
        total_particles / world_cells.len() as u32
    };
    (
        world_pressure_pa(config, average_particles),
        total_particles,
    )
}

fn remove_particles_from_world_cells(
    gas: &mut GasField,
    world_cells: &[UVec2],
    requested_amount: u32,
    world: &WorldGrid,
) -> Vec<u32> {
    if requested_amount == 0 || world_cells.is_empty() {
        return vec![0; gas.gas_count()];
    }
    let requests = world_cells
        .iter()
        .map(|cell| gas.total_amount_rounded(cell.x, cell.y))
        .collect::<Vec<_>>();
    let total_available: u32 = requests.iter().copied().sum();
    let accepted = split_bounded_integer_requests(requested_amount.min(total_available), &requests);
    let mut removed_total = vec![0u32; gas.gas_count()];
    for (cell, amount) in world_cells.iter().zip(accepted) {
        if amount == 0 {
            continue;
        }
        let removed = gas.remove_particles_proportional_counts(cell.x, cell.y, amount, world);
        add_species_counts(&mut removed_total, &removed);
    }
    removed_total
}

fn add_particles_to_world_cells(
    gas: &mut GasField,
    world_cells: &[UVec2],
    species_counts: &[u32],
    world: &WorldGrid,
) -> bool {
    if world_cells.is_empty() || species_counts.iter().all(|count| *count == 0) {
        return false;
    }
    let equal_weights = vec![1.0; world_cells.len()];
    let mut changed = false;
    for (gas_index, amount) in species_counts.iter().copied().enumerate() {
        if amount == 0 {
            continue;
        }
        let split = split_integer_by_weights(amount, &equal_weights);
        for (cell, cell_amount) in world_cells.iter().zip(split) {
            if cell_amount == 0 {
                continue;
            }
            if gas.add_particles_no_impulse(cell.x, cell.y, gas_index, cell_amount, world) > 0 {
                changed = true;
            }
        }
    }
    changed
}

fn split_bounded_integer_requests(total: u32, requests: &[u32]) -> Vec<u32> {
    if requests.is_empty() || total == 0 {
        return vec![0; requests.len()];
    }

    let total_requested: u64 = requests.iter().map(|value| u64::from(*value)).sum();
    if total_requested == 0 {
        return vec![0; requests.len()];
    }

    if total >= total_requested as u32 {
        return requests.to_vec();
    }

    let mut accepted = vec![0u32; requests.len()];
    let mut remainders = Vec::new();
    let mut used = 0u32;
    for (index, request) in requests.iter().copied().enumerate() {
        if request == 0 {
            continue;
        }
        let scaled = u64::from(total) * u64::from(request);
        let base = (scaled / total_requested) as u32;
        let bounded = base.min(request);
        accepted[index] = bounded;
        used = used.saturating_add(bounded);
        remainders.push((index, scaled % total_requested));
    }

    if used < total {
        remainders.sort_by(|(index_a, rem_a), (index_b, rem_b)| {
            rem_b.cmp(rem_a).then_with(|| index_a.cmp(index_b))
        });
        let mut remaining = total - used;
        for (index, _) in remainders {
            if remaining == 0 {
                break;
            }
            if accepted[index] >= requests[index] {
                continue;
            }
            accepted[index] += 1;
            remaining -= 1;
        }
    }

    accepted
}

fn split_integer_by_weights(total: u32, weights: &[f32]) -> Vec<u32> {
    if weights.is_empty() || total == 0 {
        return vec![0; weights.len()];
    }
    let total_weight: f32 = weights.iter().copied().sum();
    if total_weight <= f32::EPSILON {
        return vec![0; weights.len()];
    }

    let mut split = vec![0u32; weights.len()];
    let mut remainders = Vec::new();
    let mut used = 0u32;
    for (index, weight) in weights.iter().copied().enumerate() {
        if weight <= 0.0 {
            continue;
        }
        let exact = total as f32 * (weight / total_weight);
        let base = exact.floor() as u32;
        split[index] = base;
        used = used.saturating_add(base);
        remainders.push((index, exact - base as f32));
    }

    if used < total {
        remainders.sort_by(|(index_a, rem_a), (index_b, rem_b)| {
            rem_b
                .partial_cmp(rem_a)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| index_a.cmp(index_b))
        });
        let mut remaining = total - used;
        for (index, _) in remainders {
            if remaining == 0 {
                break;
            }
            split[index] += 1;
            remaining -= 1;
        }
    }

    split
}
