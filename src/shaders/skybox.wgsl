

var<push_constant> world: mat4x4<f32>;
@group(0) @binding(0) var<uniform> view_projection: mat4x4<f32>;
@group(0) @binding(1) var<uniform> camera_position: vec3<f32>;

@group(1) @binding(0) var cubemap: texture_cube<f32>;
@group(1) @binding(1) var cubemap_sampler: sampler;

fn local_to_clip(pos: vec3<f32>) -> vec4<f32> {
    return view_projection * (world * vec4<f32>(pos, 1.0));
}

fn local_to_world(pos: vec3<f32>) -> vec3<f32> {
    return (world * vec4<f32>(pos, 1.0)).xyz;
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) layer: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(1) direction: vec3<f32>,
};

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = local_to_world(in.position);
    out.direction = normalize(world_pos - camera_position);
    out.clip_position = local_to_clip(in.position);
    return out;
}

fn fix_seams(direction: vec3<f32>) -> vec3<f32> {
    let m = max(max(abs(direction.x),abs(direction.y)), abs(direction.z));
    return direction / m;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // var dir = fix_seams(in.direction);
    var dir = in.direction;
    // dir.y = dir.y * 5.0;
    const V: f32 = 1.0 / 1e-18;
    let far_sample = textureSample(cubemap, cubemap_sampler, dir);
    return far_sample;
}