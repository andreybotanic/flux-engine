struct Params {
    width: u32,
    height: u32,
    step: u32,
    gas_count: u32,
    thermal_motion_scale: f32,
    _pad0a: u32,
    _pad0b: u32,
    _pad0c: u32,
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
    _pad7: f32,
    _pad8: f32,
    _pad9: f32,
    _pad10: f32,
};

struct SpeciesBuffer {
    data: array<u32>,
};
struct U32Buffer {
    data: array<u32>,
};
struct ScalarFBuffer {
    data: array<f32>,
};
struct Vec2Buffer {
    data: array<vec2<f32>>,
};

@group(0) @binding(0) var<storage, read> species_read: SpeciesBuffer;
@group(0) @binding(1) var<storage, read_write> species_write: SpeciesBuffer;
@group(0) @binding(2) var<storage, read_write> shares: U32Buffer;
@group(0) @binding(3) var<storage, read_write> total_density: ScalarFBuffer;
@group(0) @binding(4) var<storage, read_write> velocity: Vec2Buffer;
@group(0) @binding(5) var<storage, read> solid_mask: U32Buffer;
@group(0) @binding(6) var<uniform> params: Params;
@group(0) @binding(7) var<storage, read> molecular_masses: ScalarFBuffer;

const EPSILON: f32 = 1e-6;
const BUOYANCY_MIN_ENV_MASS: f32 = 1e-6;
const DIR_X: array<i32, 4> = array<i32, 4>(0, 0, -1, 1);
const DIR_Y: array<i32, 4> = array<i32, 4>(1, -1, 0, 0);

struct Shares5 {
    v: array<u32, 5>,
};

fn world_cells() -> u32 {
    return params.width * params.height;
}

fn gas_count() -> u32 {
    return max(params.gas_count, 1u);
}

fn is_outside_i32(x: i32, y: i32) -> bool {
    return x < 0 || y < 0 || x >= i32(params.width) || y >= i32(params.height);
}

fn is_boundary_u32(x: u32, y: u32) -> bool {
    return x == 0u || y == 0u || x == params.width - 1u || y == params.height - 1u;
}

fn cell_index(x: i32, y: i32) -> u32 {
    return u32(y) * params.width + u32(x);
}

fn cell_xy(idx: u32) -> vec2<i32> {
    let y = i32(idx / params.width);
    let x = i32(idx - u32(y) * params.width);
    return vec2<i32>(x, y);
}

fn is_open_cell_i32(x: i32, y: i32) -> bool {
    if (is_outside_i32(x, y)) {
        return false;
    }
    let ux = u32(x);
    let uy = u32(y);
    if (is_boundary_u32(ux, uy)) {
        return false;
    }
    return solid_mask.data[cell_index(x, y)] == 0u;
}

fn species_offset(idx: u32, g: u32) -> u32 {
    return idx * gas_count() + g;
}

fn share_offset(idx: u32, g: u32, slot: u32) -> u32 {
    return (idx * gas_count() + g) * 5u + slot;
}

fn species_count_u32(idx: u32, g: u32) -> u32 {
    return species_read.data[species_offset(idx, g)];
}

fn set_species_count_u32(idx: u32, g: u32, value: u32) {
    species_write.data[species_offset(idx, g)] = value;
}

fn share_value(idx: u32, g: u32, slot: u32) -> u32 {
    return shares.data[share_offset(idx, g, slot)];
}

fn set_share_value(idx: u32, g: u32, slot: u32, value: u32) {
    shares.data[share_offset(idx, g, slot)] = value;
}

fn molecular_mass_for(g: u32) -> f32 {
    if (g < gas_count()) {
        return molecular_masses.data[g];
    }
    return 1.0;
}

fn sat_add_u32(a: u32, b: u32) -> u32 {
    let maxv = 0xFFFFFFFFu;
    if (a > maxv - b) {
        return maxv;
    }
    return a + b;
}

fn is_finite_f32(v: f32) -> bool {
    return v == v && abs(v) <= 3.402823466e38;
}

fn rng_seed(step: u32, idx: u32, g: u32) -> u32 {
    return step * 0xD6E8FD50u ^ idx * 0x9E3779B9u ^ g * 0x85EBCA77u;
}

fn rng_init(seed: u32) -> u32 {
    return seed ^ 0x9E3779B9u;
}

fn rng_next_u32(state: u32) -> u32 {
    var x = state;
    x = x ^ (x << 13u);
    x = x ^ (x >> 17u);
    x = x ^ (x << 5u);
    return x;
}

fn weighted_pick5(
    w0: f32,
    w1: f32,
    w2: f32,
    w3: f32,
    w4: f32,
    sum: f32,
    state: u32,
) -> vec2<u32> {
    let next = rng_next_u32(state);
    let rnd = f32(next) / 4294967296.0;
    let next_state = next;
    var t = rnd * sum;
    var fallback = 4u;

    if (w0 > 0.0) {
        fallback = 0u;
        if (t < w0) {
            return vec2<u32>(0u, next_state);
        }
    }
    t = t - w0;

    if (w1 > 0.0) {
        fallback = 1u;
        if (t < w1) {
            return vec2<u32>(1u, next_state);
        }
    }
    t = t - w1;

    if (w2 > 0.0) {
        fallback = 2u;
        if (t < w2) {
            return vec2<u32>(2u, next_state);
        }
    }
    t = t - w2;

    if (w3 > 0.0) {
        fallback = 3u;
        if (t < w3) {
            return vec2<u32>(3u, next_state);
        }
    }
    t = t - w3;

    if (w4 > 0.0) {
        fallback = 4u;
        if (t < w4) {
            return vec2<u32>(4u, next_state);
        }
    }
    return vec2<u32>(fallback, next_state);
}

fn split_count_by_weights(
    count: u32,
    w0: f32,
    w1: f32,
    w2: f32,
    w3: f32,
    w4: f32,
    seed: u32,
) -> Shares5 {
    var out: Shares5;
    out.v = array<u32, 5>(0u, 0u, 0u, 0u, 0u);
    if (count == 0u) {
        return out;
    }

    let weights_in = array<f32, 5>(w0, w1, w2, w3, w4);
    var weights = array<f32, 5>(0.0, 0.0, 0.0, 0.0, 0.0);
    var positive_total = 0.0;
    for (var i: u32 = 0u; i < 5u; i = i + 1u) {
        weights[i] = max(weights_in[i], 0.0);
        positive_total = positive_total + weights[i];
    }
    positive_total = max(positive_total, EPSILON);

    var fracs = array<f32, 5>(0.0, 0.0, 0.0, 0.0, 0.0);
    var base_sum = 0u;
    for (var i: u32 = 0u; i < 5u; i = i + 1u) {
        let raw = f32(count) * weights[i] / positive_total;
        let base = u32(floor(raw));
        out.v[i] = base;
        fracs[i] = max(raw - f32(base), 0.0);
        base_sum = sat_add_u32(base_sum, base);
    }

    var state = rng_init(seed);
    var remaining = count - min(count, base_sum);
    loop {
        if (remaining == 0u) {
            break;
        }
        var frac_sum = 0.0;
        for (var i: u32 = 0u; i < 5u; i = i + 1u) {
            frac_sum = frac_sum + fracs[i];
        }
        var pick_result = vec2<u32>(4u, state);
        if (frac_sum > EPSILON) {
            pick_result = weighted_pick5(
                fracs[0u],
                fracs[1u],
                fracs[2u],
                fracs[3u],
                fracs[4u],
                frac_sum,
                state,
            );
        } else {
            var ws_sum = 0.0;
            for (var i: u32 = 0u; i < 5u; i = i + 1u) {
                ws_sum = ws_sum + weights[i];
            }
            ws_sum = max(ws_sum, EPSILON);
            pick_result = weighted_pick5(
                weights[0u],
                weights[1u],
                weights[2u],
                weights[3u],
                weights[4u],
                ws_sum,
                state,
            );
        }
        let pick = pick_result.x;
        state = pick_result.y;
        out.v[pick] = sat_add_u32(out.v[pick], 1u);
        fracs[pick] = 0.0;
        remaining = remaining - 1u;
    }
    return out;
}

fn estimate_local_env_mix_mass(center_x: i32, center_y: i32, gc: u32) -> f32 {
    let radius = i32(clamp(params.buoyancy_window_radius, 1u, 3u));
    let width_local = u32(radius * 2 + 1);
    let height_local = width_local;
    let sx = u32(radius);
    let sy = u32(radius);

    if (!is_open_cell_i32(center_x, center_y)) {
        return -1.0;
    }

    var visited = array<u32, 64>();
    for (var i: u32 = 0u; i < 64u; i = i + 1u) {
        visited[i] = 0u;
    }
    var queue = array<u32, 64>();
    var head = 0u;
    var tail = 0u;

    let start_idx = sy * width_local + sx;
    visited[start_idx] = 1u;
    queue[tail] = start_idx;
    tail = tail + 1u;

    loop {
        if (head >= tail) {
            break;
        }
        let current = queue[head];
        head = head + 1u;
        let cx = current % width_local;
        let cy = current / width_local;

        if (cy > 0u) {
            let ny = cy - 1u;
            let nidx = ny * width_local + cx;
            if (visited[nidx] == 0u) {
                let wx = center_x + i32(cx) - radius;
                let wy = center_y + i32(ny) - radius;
                if (is_open_cell_i32(wx, wy)) {
                    visited[nidx] = 1u;
                    queue[tail] = nidx;
                    tail = tail + 1u;
                }
            }
        }
        if (cy + 1u < height_local) {
            let ny = cy + 1u;
            let nidx = ny * width_local + cx;
            if (visited[nidx] == 0u) {
                let wx = center_x + i32(cx) - radius;
                let wy = center_y + i32(ny) - radius;
                if (is_open_cell_i32(wx, wy)) {
                    visited[nidx] = 1u;
                    queue[tail] = nidx;
                    tail = tail + 1u;
                }
            }
        }
        if (cx > 0u) {
            let nx = cx - 1u;
            let nidx = cy * width_local + nx;
            if (visited[nidx] == 0u) {
                let wx = center_x + i32(nx) - radius;
                let wy = center_y + i32(cy) - radius;
                if (is_open_cell_i32(wx, wy)) {
                    visited[nidx] = 1u;
                    queue[tail] = nidx;
                    tail = tail + 1u;
                }
            }
        }
        if (cx + 1u < width_local) {
            let nx = cx + 1u;
            let nidx = cy * width_local + nx;
            if (visited[nidx] == 0u) {
                let wx = center_x + i32(nx) - radius;
                let wy = center_y + i32(cy) - radius;
                if (is_open_cell_i32(wx, wy)) {
                    visited[nidx] = 1u;
                    queue[tail] = nidx;
                    tail = tail + 1u;
                }
            }
        }
    }

    let sigma = clamp(params.buoyancy_window_sigma, 0.5, 3.0);
    let inv_two_sigma_sq = 1.0 / (2.0 * sigma * sigma);
    var weighted_mass_sum = 0.0;
    var weighted_rho_sum = 0.0;

    for (var dy: i32 = -radius; dy <= radius; dy = dy + 1) {
        for (var dx: i32 = -radius; dx <= radius; dx = dx + 1) {
            if (dx == 0 && dy == 0) {
                continue;
            }
            let lx = u32(dx + radius);
            let ly = u32(dy + radius);
            let local_idx = ly * width_local + lx;
            if (visited[local_idx] == 0u) {
                continue;
            }

            let nx = center_x + dx;
            let ny = center_y + dy;
            if (!is_open_cell_i32(nx, ny)) {
                continue;
            }

            let nidx = cell_index(nx, ny);
            var rho_n = 0.0;
            for (var gas_i: u32 = 0u; gas_i < gc; gas_i = gas_i + 1u) {
                rho_n = rho_n + f32(species_count_u32(nidx, gas_i));
            }
            if (rho_n <= EPSILON) {
                continue;
            }

            var m_mix_n = 0.0;
            for (var gas_i: u32 = 0u; gas_i < gc; gas_i = gas_i + 1u) {
                let ci = f32(species_count_u32(nidx, gas_i));
                if (ci <= EPSILON) {
                    continue;
                }
                let molecular_mass = molecular_mass_for(gas_i);
                m_mix_n = m_mix_n + (ci / rho_n) * molecular_mass;
            }
            if (!is_finite_f32(m_mix_n) || m_mix_n <= BUOYANCY_MIN_ENV_MASS) {
                continue;
            }

            let dist2 = f32(dx * dx + dy * dy);
            let w = exp(-dist2 * inv_two_sigma_sq);
            let wrho = w * rho_n;
            weighted_rho_sum = weighted_rho_sum + wrho;
            weighted_mass_sum = weighted_mass_sum + wrho * m_mix_n;
        }
    }

    if (weighted_rho_sum <= EPSILON) {
        return -1.0;
    }
    let m_env = weighted_mass_sum / weighted_rho_sum;
    if (is_finite_f32(m_env) && m_env > BUOYANCY_MIN_ENV_MASS) {
        return m_env;
    }
    return -1.0;
}

@compute @workgroup_size(64, 1, 1)
fn compute_shares(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= world_cells()) {
        return;
    }

    let gc = gas_count();
    for (var g: u32 = 0u; g < gc; g = g + 1u) {
        for (var slot: u32 = 0u; slot < 5u; slot = slot + 1u) {
            set_share_value(idx, g, slot, 0u);
        }
    }

    let xy = cell_xy(idx);
    let ux = u32(xy.x);
    let uy = u32(xy.y);
    if (is_boundary_u32(ux, uy) || solid_mask.data[idx] != 0u) {
        return;
    }

    let mobility = max(params.thermal_motion_scale, 0.0);
    let use_buoyancy = params.enable_buoyancy != 0u;
    let m_env = select(-1.0, estimate_local_env_mix_mass(xy.x, xy.y, gc), use_buoyancy);

    for (var g: u32 = 0u; g < gc; g = g + 1u) {
        let count = species_count_u32(idx, g);
        if (count == 0u) {
            continue;
        }

        let up_open = is_open_cell_i32(xy.x, xy.y + 1);
        let down_open = is_open_cell_i32(xy.x, xy.y - 1);
        let left_open = is_open_cell_i32(xy.x - 1, xy.y);
        let right_open = is_open_cell_i32(xy.x + 1, xy.y);

        var w0 = 0.0;
        var w1 = 0.0;
        var w2 = 0.0;
        var w3 = 0.0;
        var w4 = 1.0;
        let half_mobility = mobility * 0.5;

        // Direction up (0): blocked flow is redistributed along wall (left/right).
        // If only one tangent side is open (corner-like case), keep half in place
        // to avoid systematic corner drainage.
        if (up_open) {
            w0 = w0 + mobility;
        } else if (left_open && right_open) {
            w2 = w2 + half_mobility;
            w3 = w3 + half_mobility;
        } else if (left_open) {
            w2 = w2 + half_mobility;
            w4 = w4 + half_mobility;
        } else if (right_open) {
            w3 = w3 + half_mobility;
            w4 = w4 + half_mobility;
        } else {
            w4 = w4 + mobility;
        }

        // Direction down (1): blocked flow is redistributed along wall (left/right).
        // If only one tangent side is open (corner-like case), keep half in place
        // to avoid systematic corner drainage.
        if (down_open) {
            w1 = w1 + mobility;
        } else if (left_open && right_open) {
            w2 = w2 + half_mobility;
            w3 = w3 + half_mobility;
        } else if (left_open) {
            w2 = w2 + half_mobility;
            w4 = w4 + half_mobility;
        } else if (right_open) {
            w3 = w3 + half_mobility;
            w4 = w4 + half_mobility;
        } else {
            w4 = w4 + mobility;
        }

        // Direction left (2): blocked flow is redistributed along wall (up/down).
        // If only one tangent side is open (corner-like case), keep half in place
        // to avoid systematic corner drainage.
        if (left_open) {
            w2 = w2 + mobility;
        } else if (up_open && down_open) {
            w0 = w0 + half_mobility;
            w1 = w1 + half_mobility;
        } else if (up_open) {
            w0 = w0 + half_mobility;
            w4 = w4 + half_mobility;
        } else if (down_open) {
            w1 = w1 + half_mobility;
            w4 = w4 + half_mobility;
        } else {
            w4 = w4 + mobility;
        }

        // Direction right (3): blocked flow is redistributed along wall (up/down).
        // If only one tangent side is open (corner-like case), keep half in place
        // to avoid systematic corner drainage.
        if (right_open) {
            w3 = w3 + mobility;
        } else if (up_open && down_open) {
            w0 = w0 + half_mobility;
            w1 = w1 + half_mobility;
        } else if (up_open) {
            w0 = w0 + half_mobility;
            w4 = w4 + half_mobility;
        } else if (down_open) {
            w1 = w1 + half_mobility;
            w4 = w4 + half_mobility;
        } else {
            w4 = w4 + mobility;
        }

        if (m_env > BUOYANCY_MIN_ENV_MASS && mobility > 0.0) {
            let alpha = max(params.buoyancy_alpha, 0.0);
            let gain = max(params.buoyancy_gain, 0.0);
            let molecular_mass = molecular_mass_for(g);
            let xi = (m_env - molecular_mass) / m_env;
            let bi = tanh(gain * xi) * pow(abs(xi), alpha);
            let cap = abs(params.buoyancy_force_cap);
            let bias = clamp(params.buoyancy_strength * bi, -cap, cap) * mobility;
            if (up_open) {
                w0 = max(w0 + max(bias, 0.0), 0.0);
            }
            if (down_open) {
                w1 = max(w1 + max(-bias, 0.0), 0.0);
            }
        }

        let seed = rng_seed(params.step, idx, g);
        let split = split_count_by_weights(count, w0, w1, w2, w3, w4, seed);
        for (var slot: u32 = 0u; slot < 5u; slot = slot + 1u) {
            set_share_value(idx, g, slot, split.v[slot]);
        }
    }
}

@compute @workgroup_size(64, 1, 1)
fn step_discrete_exact(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= world_cells()) {
        return;
    }

    let xy = cell_xy(idx);
    let ux = u32(xy.x);
    let uy = u32(xy.y);
    let gc = gas_count();

    if (is_boundary_u32(ux, uy) || solid_mask.data[idx] != 0u) {
        for (var g: u32 = 0u; g < gc; g = g + 1u) {
            set_species_count_u32(idx, g, 0u);
        }
        total_density.data[idx] = 0.0;
        velocity.data[idx] = vec2<f32>(0.0, 0.0);
        return;
    }

    var net_momentum = vec2<f32>(0.0, 0.0);
    var mass = 0.0;

    for (var g: u32 = 0u; g < gc; g = g + 1u) {
        var out_count = share_value(idx, g, 4u);

        for (var d: u32 = 0u; d < 4u; d = d + 1u) {
            let moved = share_value(idx, g, d);
            if (moved > 0u) {
                net_momentum =
                    net_momentum - vec2<f32>(f32(DIR_X[d]), f32(DIR_Y[d])) * f32(moved);
            }
        }

        let src0_x = xy.x;
        let src0_y = xy.y - 1;
        let src1_x = xy.x;
        let src1_y = xy.y + 1;
        let src2_x = xy.x + 1;
        let src2_y = xy.y;
        let src3_x = xy.x - 1;
        let src3_y = xy.y;

        if (is_open_cell_i32(src0_x, src0_y)) {
            let sidx = cell_index(src0_x, src0_y);
            let moved = share_value(sidx, g, 0u);
            out_count = sat_add_u32(out_count, moved);
            if (moved > 0u) {
                net_momentum = net_momentum + vec2<f32>(0.0, 1.0) * f32(moved);
            }
        }
        if (is_open_cell_i32(src1_x, src1_y)) {
            let sidx = cell_index(src1_x, src1_y);
            let moved = share_value(sidx, g, 1u);
            out_count = sat_add_u32(out_count, moved);
            if (moved > 0u) {
                net_momentum = net_momentum + vec2<f32>(0.0, -1.0) * f32(moved);
            }
        }
        if (is_open_cell_i32(src2_x, src2_y)) {
            let sidx = cell_index(src2_x, src2_y);
            let moved = share_value(sidx, g, 2u);
            out_count = sat_add_u32(out_count, moved);
            if (moved > 0u) {
                net_momentum = net_momentum + vec2<f32>(-1.0, 0.0) * f32(moved);
            }
        }
        if (is_open_cell_i32(src3_x, src3_y)) {
            let sidx = cell_index(src3_x, src3_y);
            let moved = share_value(sidx, g, 3u);
            out_count = sat_add_u32(out_count, moved);
            if (moved > 0u) {
                net_momentum = net_momentum + vec2<f32>(1.0, 0.0) * f32(moved);
            }
        }

        set_species_count_u32(idx, g, out_count);
        mass = mass + f32(out_count);
    }

    total_density.data[idx] = mass;
    if (mass > 0.0) {
        velocity.data[idx] = net_momentum / mass;
    } else {
        velocity.data[idx] = vec2<f32>(0.0, 0.0);
    }
}
