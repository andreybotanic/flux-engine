#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct PipeHighlightMaterial {
    tint: vec4<f32>,
    params: vec4<f32>,
}

@group(2) @binding(0) var<uniform> material: PipeHighlightMaterial;
@group(2) @binding(1) var color_texture: texture_2d<f32>;
@group(2) @binding(2) var color_sampler: sampler;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let sample = textureSample(color_texture, color_sampler, mesh.uv);
    if (sample.a <= 0.001) {
        discard;
    }

    let gain = material.params.x;
    let curve = material.params.y;
    let bias = material.params.z;
    let opacity = material.params.w;

    let luminance = dot(sample.rgb, vec3(0.2126, 0.7152, 0.0722));
    let strength = clamp(pow(max(luminance, 0.0001), curve) * gain + bias, 0.0, 1.0);
    let alpha = sample.a * strength * opacity;

    return vec4(material.tint.rgb, alpha * material.tint.a);
}
