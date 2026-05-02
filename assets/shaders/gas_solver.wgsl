struct Params {
    width: u32,
    height: u32,
    step: u32,
    gas_count: u32,

    enable_species_relaxation: u32,
    enable_lbm_velocity: u32,
    enable_buoyancy: u32,
    _pad1: u32,

    tau_even: f32,
    tau_odd: f32,
    target_cfl_like_limit: f32,
    buoyancy_strength: f32,

    buoyancy_window_radius: u32,
    _pad2: u32,
    _pad3: u32,
    _pad4: u32,

    buoyancy_window_sigma: f32,
    buoyancy_gain: f32,
    buoyancy_alpha: f32,
    buoyancy_force_cap: f32,

    reconcile_every_n_steps: u32,
    mass_fix_every_n_steps: u32,
    _pad5: u32,
    _pad6: u32,

    mass_fix_error_threshold: f32,
    mass_fix_min_residual: f32,
    molecular_mass_h2: f32,
    molecular_mass_o2: f32,
    molecular_mass_co2: f32,
    _pad8: f32,
};

struct SpeciesBuffer {
    data: array<vec4<f32>>,
};

struct ScalarBuffer {
    data: array<f32>,
};

struct Vec2Buffer {
    data: array<vec2<f32>>,
};

struct U32Buffer {
    data: array<u32>,
};

struct Vec4Buffer {
    data: array<vec4<f32>>,
};

@group(0) @binding(0) var<storage, read_write> species_read: SpeciesBuffer;
@group(0) @binding(1) var<storage, read_write> species_write: SpeciesBuffer;
@group(0) @binding(2) var<storage, read_write> lbm_read: ScalarBuffer;
@group(0) @binding(3) var<storage, read_write> lbm_write: ScalarBuffer;
@group(0) @binding(4) var<storage, read_write> total_density: ScalarBuffer;
@group(0) @binding(5) var<storage, read_write> velocity: Vec2Buffer;
@group(0) @binding(6) var<storage, read> solid_mask: U32Buffer;
@group(0) @binding(7) var<uniform> params: Params;
@group(0) @binding(8) var<storage, read_write> reduce_meta: Vec4Buffer;
@group(0) @binding(9) var<storage, read_write> species_meta: Vec4Buffer;
@group(0) @binding(10) var<storage, read_write> lbm_post: ScalarBuffer;

const EPSILON_DENSITY: f32 = 1e-6;
const LBM_VELOCITY_CLAMP: f32 = 0.95;
const SPECIES_RELATIVE_DRIFT_SCALE: f32 = 0.45;
const BUOYANCY_MIN_ENV_MASS: f32 = 1e-6;
const REDUCE_PARTIAL_OFFSET: u32 = 4u;
var<workgroup> reduce_sums_target: array<vec3<f32>, 64>;
var<workgroup> reduce_sums_current: array<vec3<f32>, 64>;

const DIR_X: array<i32, 9> = array<i32, 9>(0, 1, -1, 0, 0, 1, -1, -1, 1);
const DIR_Y: array<i32, 9> = array<i32, 9>(0, 0, 0, 1, -1, 1, 1, -1, -1);
const DIR_X_F: array<f32, 9> = array<f32, 9>(0.0, 1.0, -1.0, 0.0, 0.0, 1.0, -1.0, -1.0, 1.0);
const DIR_Y_F: array<f32, 9> = array<f32, 9>(0.0, 0.0, 0.0, 1.0, -1.0, 1.0, 1.0, -1.0, -1.0);
const LBM_WEIGHTS: array<f32, 9> = array<f32, 9>(
    4.0 / 9.0,
    1.0 / 9.0,
    1.0 / 9.0,
    1.0 / 9.0,
    1.0 / 9.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0,
    1.0 / 36.0
);
const OPPOSITE: array<u32, 9> = array<u32, 9>(0u, 2u, 1u, 4u, 3u, 7u, 8u, 5u, 6u);

fn world_cells() -> u32 {
    return params.width * params.height;
}

fn active_gas_count() -> u32 {
    return clamp(params.gas_count, 1u, 3u);
}

fn lbm_idx(cell: u32, dir: u32) -> u32 {
    return cell * 9u + dir;
}

fn is_outside_i32(x: i32, y: i32) -> bool {
    return x < 0 || y < 0 || x >= i32(params.width) || y >= i32(params.height);
}

fn cell_xy(idx: u32) -> vec2<i32> {
    let y = i32(idx / params.width);
    let x = i32(idx - u32(y) * params.width);
    return vec2<i32>(x, y);
}

fn cell_index(x: i32, y: i32) -> u32 {
    return u32(y) * params.width + u32(x);
}

fn molecular_mass(kind: u32) -> f32 {
    if (kind == 0u) {
        return params.molecular_mass_h2;
    }
    if (kind == 1u) {
        return params.molecular_mass_o2;
    }
    return params.molecular_mass_co2;
}

fn gas_amount(s: vec4<f32>, kind: u32) -> f32 {
    if (kind == 0u) {
        return max(s.x, 0.0);
    }
    if (kind == 1u) {
        return max(s.y, 0.0);
    }
    return max(s.z, 0.0);
}

fn clamp_vec_len(v: vec2<f32>, limit: f32) -> vec2<f32> {
    let len_sq = dot(v, v);
    let lim_sq = limit * limit;
    if (len_sq <= lim_sq || len_sq <= 1e-20) {
        return v;
    }
    let len = sqrt(len_sq);
    return v * (limit / len);
}

fn species_sum(s: vec4<f32>) -> f32 {
    return max(s.x, 0.0) + max(s.y, 0.0) + max(s.z, 0.0);
}

fn macroscopic_from_lbm_read(cell: u32) -> vec3<f32> {
    var rho = 0.0;
    var mx = 0.0;
    var my = 0.0;
    for (var d: u32 = 0u; d < 9u; d = d + 1u) {
        let f = lbm_read.data[lbm_idx(cell, d)];
        rho = rho + f;
        mx = mx + f * DIR_X_F[d];
        my = my + f * DIR_Y_F[d];
    }
    if (rho <= EPSILON_DENSITY) {
        return vec3<f32>(0.0, 0.0, 0.0);
    }
    return vec3<f32>(rho, mx / rho, my / rho);
}

fn estimate_local_env_mix_mass(x: i32, y: i32) -> vec2<f32> {
    let radius = i32(clamp(f32(params.buoyancy_window_radius), 1.0, 3.0));
    let sigma = clamp(params.buoyancy_window_sigma, 0.5, 3.0);
    let inv_two_sigma_sq = 1.0 / (2.0 * sigma * sigma);

    var weighted_mass_sum = 0.0;
    var weighted_rho_sum = 0.0;

    for (var dy: i32 = -radius; dy <= radius; dy = dy + 1) {
        for (var dx: i32 = -radius; dx <= radius; dx = dx + 1) {
            if (dx == 0 && dy == 0) {
                continue;
            }
            let nx = x + dx;
            let ny = y + dy;
            if (is_outside_i32(nx, ny)) {
                continue;
            }

            let nidx = cell_index(nx, ny);
            if (solid_mask.data[nidx] != 0u) {
                continue;
            }

            let s = species_read.data[nidx];
            let rho_n = species_sum(s);
            if (rho_n <= EPSILON_DENSITY) {
                continue;
            }

            var m_mix_n = 0.0;
            let gas_count = active_gas_count();
            for (var kind: u32 = 0u; kind < gas_count; kind = kind + 1u) {
                let amount = gas_amount(s, kind);
                if (amount > EPSILON_DENSITY) {
                    m_mix_n = m_mix_n + (amount / rho_n) * molecular_mass(kind);
                }
            }
            if (!(m_mix_n > BUOYANCY_MIN_ENV_MASS)) {
                continue;
            }

            let dist2 = f32(dx * dx + dy * dy);
            let w = exp(-dist2 * inv_two_sigma_sq);
            let wrho = w * rho_n;
            weighted_rho_sum = weighted_rho_sum + wrho;
            weighted_mass_sum = weighted_mass_sum + wrho * m_mix_n;
        }
    }

    if (weighted_rho_sum <= EPSILON_DENSITY) {
        return vec2<f32>(0.0, 0.0);
    }
    let m_env = weighted_mass_sum / weighted_rho_sum;
    if (m_env > BUOYANCY_MIN_ENV_MASS) {
        return vec2<f32>(1.0, m_env);
    }
    return vec2<f32>(0.0, 0.0);
}

fn equilibrium_distributions(rho: f32, vel: vec2<f32>, dir: u32) -> f32 {
    let ux = vel.x;
    let uy = vel.y;
    let cx = DIR_X_F[dir];
    let cy = DIR_Y_F[dir];
    let cu = cx * ux + cy * uy;
    let u_sq = ux * ux + uy * uy;
    let v = LBM_WEIGHTS[dir] * rho * (1.0 + 3.0 * cu + 4.5 * cu * cu - 1.5 * u_sq);
    return max(v, 0.0);
}

@compute @workgroup_size(64, 1, 1)
fn reduce_species_target_partial(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(workgroup_id) wid: vec3<u32>
) {
    let idx = gid.x;
    var value = vec3<f32>(0.0, 0.0, 0.0);
    if (idx < world_cells() && solid_mask.data[idx] == 0u) {
        let s = species_read.data[idx];
        value = vec3<f32>(max(s.x, 0.0), max(s.y, 0.0), max(s.z, 0.0));
    }
    reduce_sums_target[lid.x] = value;
    workgroupBarrier();

    var stride: u32 = 32u;
    loop {
        if (lid.x < stride) {
            reduce_sums_target[lid.x] = reduce_sums_target[lid.x] + reduce_sums_target[lid.x + stride];
        }
        workgroupBarrier();
        if (stride == 1u) {
            break;
        }
        stride = stride / 2u;
    }

    if (lid.x == 0u) {
        reduce_meta.data[REDUCE_PARTIAL_OFFSET + wid.x] = vec4<f32>(reduce_sums_target[0].x, reduce_sums_target[0].y, reduce_sums_target[0].z, 0.0);
    }
}

@compute @workgroup_size(1, 1, 1)
fn reduce_species_target_finalize(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x != 0u || gid.y != 0u || gid.z != 0u) {
        return;
    }

    let partial_count = (world_cells() + 63u) / 64u;
    var sum = vec3<f32>(0.0, 0.0, 0.0);
    for (var i: u32 = 0u; i < partial_count; i = i + 1u) {
        let v = reduce_meta.data[REDUCE_PARTIAL_OFFSET + i];
        sum = sum + v.xyz;
    }
    reduce_meta.data[0] = vec4<f32>(sum.x, sum.y, sum.z, 0.0);
}

@compute @workgroup_size(64, 1, 1)
fn compute_post_and_species_meta(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= world_cells()) {
        return;
    }

    if (solid_mask.data[idx] != 0u) {
        species_meta.data[idx] = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        for (var d: u32 = 0u; d < 9u; d = d + 1u) {
            lbm_post.data[lbm_idx(idx, d)] = 0.0;
        }
        return;
    }

    let velocity_limit = clamp(params.target_cfl_like_limit, 0.1, 0.98);
    let omega_plus = clamp(1.0 / max(params.tau_even, 0.55), 0.01, 1.99);
    let omega_minus = clamp(1.0 / max(params.tau_odd, 0.55), 0.01, 1.99);

    let macro_state = macroscopic_from_lbm_read(idx);
    let rho = macro_state.x;
    var forced_u = vec2<f32>(macro_state.y, macro_state.z);

    var local_species_b = vec3<f32>(0.0, 0.0, 0.0);
    var local_mix_b = 0.0;
    var has_context = false;

    if (params.enable_lbm_velocity == 0u) {
        forced_u = vec2<f32>(0.0, 0.0);
    } else if (params.enable_buoyancy != 0u && rho > EPSILON_DENSITY) {
        let s = species_read.data[idx];
        let species_total = species_sum(s);
        if (species_total > EPSILON_DENSITY) {
            let env = estimate_local_env_mix_mass(cell_xy(idx).x, cell_xy(idx).y);
            if (env.x > 0.5) {
                has_context = true;
                let m_env = env.y;
                let alpha = max(params.buoyancy_alpha, 0.0);
                let gain = max(params.buoyancy_gain, 0.0);
                var m_cell = 0.0;
                let gas_count = active_gas_count();
                for (var kind: u32 = 0u; kind < gas_count; kind = kind + 1u) {
                    let amount = gas_amount(s, kind);
                    if (amount > EPSILON_DENSITY) {
                        let yi = amount / species_total;
                        m_cell = m_cell + yi * molecular_mass(kind);
                        let xi = (m_env - molecular_mass(kind)) / m_env;
                        let b = tanh(gain * xi) * pow(abs(xi), alpha);
                        if (kind == 0u) {
                            local_species_b.x = b;
                        } else if (kind == 1u) {
                            local_species_b.y = b;
                        } else {
                            local_species_b.z = b;
                        }
                    }
                }

                let x_mix = (m_env - m_cell) / m_env;
                local_mix_b = tanh(gain * x_mix) * pow(abs(x_mix), alpha);
            }

            let cap = abs(params.buoyancy_force_cap);
            let force_y = clamp(params.buoyancy_strength * local_mix_b, -cap, cap);
            forced_u.y = forced_u.y + force_y;
        }
    }

    forced_u = clamp_vec_len(forced_u, velocity_limit);

    var feq: array<f32, 9>;
    for (var d: u32 = 0u; d < 9u; d = d + 1u) {
        feq[d] = equilibrium_distributions(rho, forced_u, d);
    }

    var post: array<f32, 9>;
    var post_sum = 0.0;
    for (var d: u32 = 0u; d < 9u; d = d + 1u) {
        let j = OPPOSITE[d];
        let fi = lbm_read.data[lbm_idx(idx, d)];
        let fj = lbm_read.data[lbm_idx(idx, j)];
        let feqi = feq[d];
        let feqj = feq[j];
        let f_plus = 0.5 * (fi + fj);
        let f_minus = 0.5 * (fi - fj);
        let feq_plus = 0.5 * (feqi + feqj);
        let feq_minus = 0.5 * (feqi - feqj);
        let v = max(fi - omega_plus * (f_plus - feq_plus) - omega_minus * (f_minus - feq_minus), 0.0);
        post[d] = v;
        post_sum = post_sum + v;
        lbm_post.data[lbm_idx(idx, d)] = v;
    }

    var inv_sum = 0.0;
    if (params.enable_species_relaxation != 0u && post_sum > EPSILON_DENSITY) {
        inv_sum = 1.0 / post_sum;
    }

    var drift = vec3<f32>(0.0, 0.0, 0.0);
    if (params.enable_species_relaxation != 0u && params.enable_buoyancy != 0u && has_context) {
        drift = clamp((local_species_b - vec3<f32>(local_mix_b, local_mix_b, local_mix_b))
            * params.buoyancy_strength * SPECIES_RELATIVE_DRIFT_SCALE,
            vec3<f32>(-0.35, -0.35, -0.35),
            vec3<f32>(0.35, 0.35, 0.35));
    }

    species_meta.data[idx] = vec4<f32>(inv_sum, drift.x, drift.y, drift.z);
}

fn stream_target_blocked(src_x: i32, src_y: i32, dir: u32) -> bool {
    let tx = src_x + DIR_X[dir];
    let ty = src_y + DIR_Y[dir];
    if (is_outside_i32(tx, ty)) {
        return true;
    }
    let tidx = cell_index(tx, ty);
    return solid_mask.data[tidx] != 0u;
}

fn lbm_incoming_for_target_dir(target_idx: u32, target_xy: vec2<i32>, dir: u32) -> f32 {
    var incoming = 0.0;

    let sx = target_xy.x - DIR_X[dir];
    let sy = target_xy.y - DIR_Y[dir];
    if (!is_outside_i32(sx, sy)) {
        let sidx = cell_index(sx, sy);
        if (solid_mask.data[sidx] == 0u && !stream_target_blocked(sx, sy, dir)) {
            incoming = incoming + lbm_post.data[lbm_idx(sidx, dir)];
        }
    }

    let opp = OPPOSITE[dir];
    if (stream_target_blocked(target_xy.x, target_xy.y, opp)) {
        incoming = incoming + lbm_post.data[lbm_idx(target_idx, opp)];
    }

    return incoming;
}

fn species_from_source_dir(source_idx: u32, target_idx: u32, kind: u32, dir: u32) -> f32 {
    let s = species_read.data[source_idx];
    let amount = gas_amount(s, kind);
    if (amount <= EPSILON_DENSITY) {
        return 0.0;
    }

    let species_aux = species_meta.data[source_idx];
    let inv_sum = species_aux.x;
    if (inv_sum <= EPSILON_DENSITY) {
        if (source_idx == target_idx) {
            return amount;
        }
        return 0.0;
    }

    let drift = select(species_aux.w, select(species_aux.z, species_aux.y, kind == 0u), kind == 2u);
    let mult = max(1.0 + drift * DIR_Y_F[dir], 0.0);
    let post = lbm_post.data[lbm_idx(source_idx, dir)];
    let share = amount * post * mult * inv_sum;
    if (share <= EPSILON_DENSITY) {
        return 0.0;
    }
    return share;
}

@compute @workgroup_size(64, 1, 1)
fn stream_and_species_gather(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= world_cells()) {
        return;
    }

    if (solid_mask.data[idx] != 0u) {
        species_write.data[idx] = vec4<f32>(0.0, 0.0, 0.0, 0.0);
        for (var d: u32 = 0u; d < 9u; d = d + 1u) {
            lbm_write.data[lbm_idx(idx, d)] = 0.0;
        }
        total_density.data[idx] = 0.0;
        velocity.data[idx] = vec2<f32>(0.0, 0.0);
        return;
    }

    let velocity_limit = clamp(params.target_cfl_like_limit, 0.1, 0.98);
    let xy = cell_xy(idx);

    var rho = 0.0;
    var mx = 0.0;
    var my = 0.0;
    for (var d: u32 = 0u; d < 9u; d = d + 1u) {
        let incoming = lbm_incoming_for_target_dir(idx, xy, d);
        lbm_write.data[lbm_idx(idx, d)] = incoming;
        rho = rho + incoming;
        mx = mx + incoming * DIR_X_F[d];
        my = my + incoming * DIR_Y_F[d];
    }

    if (rho > EPSILON_DENSITY) {
        velocity.data[idx] = clamp_vec_len(vec2<f32>(mx / rho, my / rho), velocity_limit);
    } else {
        velocity.data[idx] = vec2<f32>(0.0, 0.0);
    }
    total_density.data[idx] = max(rho, 0.0);

    if (params.enable_species_relaxation == 0u) {
        species_write.data[idx] = species_read.data[idx];
        return;
    }

    var accum = vec3<f32>(0.0, 0.0, 0.0);
    let gas_count = active_gas_count();

    for (var d: u32 = 0u; d < 9u; d = d + 1u) {
        let sx = xy.x - DIR_X[d];
        let sy = xy.y - DIR_Y[d];
        if (!is_outside_i32(sx, sy)) {
            let sidx = cell_index(sx, sy);
            if (solid_mask.data[sidx] == 0u && !stream_target_blocked(sx, sy, d)) {
                for (var kind: u32 = 0u; kind < gas_count; kind = kind + 1u) {
                    let share = species_from_source_dir(sidx, idx, kind, d);
                    if (kind == 0u) {
                        accum.x = accum.x + share;
                    } else if (kind == 1u) {
                        accum.y = accum.y + share;
                    } else {
                        accum.z = accum.z + share;
                    }
                }
            }
        }

        if (stream_target_blocked(xy.x, xy.y, d)) {
            for (var kind: u32 = 0u; kind < gas_count; kind = kind + 1u) {
                let share = species_from_source_dir(idx, idx, kind, d);
                if (kind == 0u) {
                    accum.x = accum.x + share;
                } else if (kind == 1u) {
                    accum.y = accum.y + share;
                } else {
                    accum.z = accum.z + share;
                }
            }
        }
    }

    species_write.data[idx] = vec4<f32>(max(accum.x, 0.0), max(accum.y, 0.0), max(accum.z, 0.0), 0.0);
}

@compute @workgroup_size(64, 1, 1)
fn update_macro_from_lbm(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= world_cells()) {
        return;
    }

    if (solid_mask.data[idx] != 0u) {
        velocity.data[idx] = vec2<f32>(0.0, 0.0);
        total_density.data[idx] = 0.0;
        return;
    }

    let velocity_limit = clamp(params.target_cfl_like_limit, 0.1, 0.98);
    var rho = 0.0;
    var mx = 0.0;
    var my = 0.0;
    for (var d: u32 = 0u; d < 9u; d = d + 1u) {
        let f = lbm_read.data[lbm_idx(idx, d)];
        rho = rho + f;
        mx = mx + f * DIR_X_F[d];
        my = my + f * DIR_Y_F[d];
    }

    if (rho > EPSILON_DENSITY) {
        velocity.data[idx] = clamp_vec_len(vec2<f32>(mx / rho, my / rho), velocity_limit);
    } else {
        velocity.data[idx] = vec2<f32>(0.0, 0.0);
    }
    total_density.data[idx] = max(rho, 0.0);
}

@compute @workgroup_size(64, 1, 1)
fn reduce_species_current_partial(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(workgroup_id) wid: vec3<u32>
) {
    let idx = gid.x;
    var value = vec3<f32>(0.0, 0.0, 0.0);
    if (idx < world_cells() && solid_mask.data[idx] == 0u) {
        let s = species_read.data[idx];
        value = vec3<f32>(max(s.x, 0.0), max(s.y, 0.0), max(s.z, 0.0));
    }
    reduce_sums_current[lid.x] = value;
    workgroupBarrier();

    var stride: u32 = 32u;
    loop {
        if (lid.x < stride) {
            reduce_sums_current[lid.x] = reduce_sums_current[lid.x] + reduce_sums_current[lid.x + stride];
        }
        workgroupBarrier();
        if (stride == 1u) {
            break;
        }
        stride = stride / 2u;
    }

    if (lid.x == 0u) {
        reduce_meta.data[REDUCE_PARTIAL_OFFSET + wid.x] = vec4<f32>(reduce_sums_current[0].x, reduce_sums_current[0].y, reduce_sums_current[0].z, 0.0);
    }
}

@compute @workgroup_size(1, 1, 1)
fn reduce_species_current_finalize(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x != 0u || gid.y != 0u || gid.z != 0u) {
        return;
    }

    let partial_count = (world_cells() + 63u) / 64u;
    var sum = vec3<f32>(0.0, 0.0, 0.0);
    for (var i: u32 = 0u; i < partial_count; i = i + 1u) {
        let v = reduce_meta.data[REDUCE_PARTIAL_OFFSET + i];
        sum = sum + v.xyz;
    }
    reduce_meta.data[1] = vec4<f32>(sum.x, sum.y, sum.z, 0.0);
}

@compute @workgroup_size(1, 1, 1)
fn compute_massfix_flags(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x != 0u || gid.y != 0u || gid.z != 0u) {
        return;
    }

    let target_totals = reduce_meta.data[0].xyz;
    let current_totals = reduce_meta.data[1].xyz;

    let err_h2 = abs(target_totals.x - current_totals.x);
    let err_o2 = abs(target_totals.y - current_totals.y);
    let err_co2 = abs(target_totals.z - current_totals.z);
    let rel_h2 = select(err_h2, err_h2 / target_totals.x, target_totals.x > 1e-6);
    let rel_o2 = select(err_o2, err_o2 / target_totals.y, target_totals.y > 1e-6);
    let rel_co2 = select(err_co2, err_co2 / target_totals.z, target_totals.z > 1e-6);
    let mass_err = max(max(rel_h2, rel_o2), rel_co2);

    let periodic_mass_fix = params.mass_fix_every_n_steps > 0u && ((params.step + 1u) % params.mass_fix_every_n_steps == 0u);
    let event_mass_fix = mass_err > max(params.mass_fix_error_threshold, 0.0);
    let do_mass_fix = periodic_mass_fix || event_mass_fix;

    var scale = vec3<f32>(1.0, 1.0, 1.0);
    if (do_mass_fix) {
        if (target_totals.x > EPSILON_DENSITY && current_totals.x > EPSILON_DENSITY) {
            scale.x = target_totals.x / current_totals.x;
        }
        if (target_totals.y > EPSILON_DENSITY && current_totals.y > EPSILON_DENSITY) {
            scale.y = target_totals.y / current_totals.y;
        }
        if (target_totals.z > EPSILON_DENSITY && current_totals.z > EPSILON_DENSITY) {
            scale.z = target_totals.z / current_totals.z;
        }
    }
    reduce_meta.data[2] = vec4<f32>(scale.x, scale.y, scale.z, select(0.0, 1.0, do_mass_fix));

    let periodic_reconcile = params.reconcile_every_n_steps > 0u && ((params.step + 1u) % params.reconcile_every_n_steps == 0u);
    let do_reconcile = params.enable_lbm_velocity != 0u && (periodic_reconcile || event_mass_fix);
    reduce_meta.data[3] = vec4<f32>(select(0.0, 1.0, do_reconcile), 0.0, 0.0, 0.0);
}

@compute @workgroup_size(64, 1, 1)
fn apply_mass_fix(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= world_cells()) {
        return;
    }

    if (solid_mask.data[idx] != 0u) {
        return;
    }

    if (reduce_meta.data[2].w < 0.5) {
        return;
    }

    let scale = reduce_meta.data[2].xyz;
    let s = species_read.data[idx];
    species_read.data[idx] = vec4<f32>(max(s.x * scale.x, 0.0), max(s.y * scale.y, 0.0), max(s.z * scale.z, 0.0), 0.0);
}

@compute @workgroup_size(64, 1, 1)
fn recompute_total_density_from_species(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= world_cells()) {
        return;
    }

    if (solid_mask.data[idx] != 0u) {
        total_density.data[idx] = 0.0;
        return;
    }

    let s = species_read.data[idx];
    total_density.data[idx] = species_sum(s);
}

@compute @workgroup_size(64, 1, 1)
fn reconcile_if_needed(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= world_cells()) {
        return;
    }

    if (reduce_meta.data[3].x < 0.5) {
        return;
    }

    if (solid_mask.data[idx] != 0u) {
        for (var d: u32 = 0u; d < 9u; d = d + 1u) {
            lbm_read.data[lbm_idx(idx, d)] = 0.0;
            lbm_write.data[lbm_idx(idx, d)] = 0.0;
        }
        total_density.data[idx] = 0.0;
        velocity.data[idx] = vec2<f32>(0.0, 0.0);
        return;
    }

    let s = species_read.data[idx];
    let species_total = species_sum(s);
    if (species_total <= EPSILON_DENSITY) {
        for (var d: u32 = 0u; d < 9u; d = d + 1u) {
            lbm_read.data[lbm_idx(idx, d)] = 0.0;
            lbm_write.data[lbm_idx(idx, d)] = 0.0;
        }
        total_density.data[idx] = 0.0;
        velocity.data[idx] = vec2<f32>(0.0, 0.0);
        return;
    }

    var lbm_total = 0.0;
    for (var d: u32 = 0u; d < 9u; d = d + 1u) {
        lbm_total = lbm_total + lbm_read.data[lbm_idx(idx, d)];
    }

    if (lbm_total <= EPSILON_DENSITY) {
        let u = clamp_vec_len(velocity.data[idx], LBM_VELOCITY_CLAMP);
        for (var d: u32 = 0u; d < 9u; d = d + 1u) {
            let feq = equilibrium_distributions(species_total, u, d);
            lbm_read.data[lbm_idx(idx, d)] = feq;
            lbm_write.data[lbm_idx(idx, d)] = feq;
        }
    } else {
        let scale = species_total / lbm_total;
        for (var d: u32 = 0u; d < 9u; d = d + 1u) {
            let v = max(lbm_read.data[lbm_idx(idx, d)] * scale, 0.0);
            lbm_read.data[lbm_idx(idx, d)] = v;
            lbm_write.data[lbm_idx(idx, d)] = v;
        }
    }

    var rho = 0.0;
    var mx = 0.0;
    var my = 0.0;
    for (var d: u32 = 0u; d < 9u; d = d + 1u) {
        let f = lbm_read.data[lbm_idx(idx, d)];
        rho = rho + f;
        mx = mx + f * DIR_X_F[d];
        my = my + f * DIR_Y_F[d];
    }

    if (rho > EPSILON_DENSITY) {
        velocity.data[idx] = clamp_vec_len(vec2<f32>(mx / rho, my / rho), LBM_VELOCITY_CLAMP);
    } else {
        velocity.data[idx] = vec2<f32>(0.0, 0.0);
    }
    total_density.data[idx] = max(rho, 0.0);
}
