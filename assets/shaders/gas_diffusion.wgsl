@group(0) @binding(0) var input_texture: texture_storage_2d<rgba32float, read>;
@group(0) @binding(1) var output_texture: texture_storage_2d<rgba32float, write>;

const DIFFUSION_NUMERATOR: u32 = 1u;
const DIFFUSION_DENOMINATOR: u32 = 5u;
const DISPLAY_MAX_PARTICLES: f32 = 220.0;
const STORAGE_MAX_PARTICLES: f32 = 10000.0;

fn is_wall(pixel: vec4<f32>) -> bool {
    return pixel.g > 0.5;
}

fn sample_pixel(location: vec2<i32>) -> vec4<f32> {
    return textureLoad(input_texture, location);
}

fn particles_from_pixel(pixel: vec4<f32>) -> u32 {
    return u32(max(pixel.b, 0.0) * STORAGE_MAX_PARTICLES + 0.5);
}

fn open_neighbour_count(location: vec2<i32>) -> u32 {
    var count = 0u;
    let offsets = array<vec2<i32>, 4>(
        vec2<i32>(-1, 0),
        vec2<i32>(1, 0),
        vec2<i32>(0, -1),
        vec2<i32>(0, 1),
    );

    for (var i = 0; i < 4; i = i + 1) {
        let neighbour = sample_pixel(location + offsets[i]);
        if (!is_wall(neighbour)) {
            count += 1u;
        }
    }

    return count;
}

@compute @workgroup_size(8, 8, 1)
fn update(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let size = textureDimensions(input_texture);
    let width = size.x;
    let height = size.y;

    if (invocation_id.x >= width || invocation_id.y >= height) {
        return;
    }

    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    let center = sample_pixel(location);

    if (is_wall(center)) {
        textureStore(output_texture, location, vec4<f32>(0.0, 1.0, 0.0, 1.0));
        return;
    }

    let center_particles = particles_from_pixel(center);
    let center_open = open_neighbour_count(location);
    let center_movable = center_particles * DIFFUSION_NUMERATOR / DIFFUSION_DENOMINATOR;
    let center_share = select(0u, center_movable / center_open, center_open > 0u);
    let center_outgoing = center_share * center_open;
    var next_particles = center_particles - center_outgoing;

    let offsets = array<vec2<i32>, 4>(
        vec2<i32>(-1, 0),
        vec2<i32>(1, 0),
        vec2<i32>(0, -1),
        vec2<i32>(0, 1),
    );

    for (var i = 0; i < 4; i = i + 1) {
        let neighbour_location = location + offsets[i];
        let neighbour = sample_pixel(neighbour_location);
        if (is_wall(neighbour)) {
            continue;
        }

        let neighbour_open = open_neighbour_count(neighbour_location);
        if (neighbour_open == 0u) {
            continue;
        }

        let neighbour_particles = particles_from_pixel(neighbour);
        let neighbour_movable = neighbour_particles * DIFFUSION_NUMERATOR / DIFFUSION_DENOMINATOR;
        next_particles += neighbour_movable / neighbour_open;
    }

    let storage_linear = min(f32(next_particles) / STORAGE_MAX_PARTICLES, 1.0);
    let visual_linear = min(f32(next_particles) / DISPLAY_MAX_PARTICLES, 1.0);
    let visual = pow(visual_linear, 0.35);
    textureStore(output_texture, location, vec4<f32>(visual, 0.0, storage_linear, 1.0));
}