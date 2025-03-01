// @group(0) @binding(0) var<uniform> world: mat4x4<f32>;
var<push_constant> world: mat4x4<f32>;
@group(0) @binding(0) var<uniform> view_projection: mat4x4<f32>;

@group(1) @binding(0) var array_texture: texture_2d_array<f32>;
@group(1) @binding(1) var array_texture_sampler: sampler;

fn local_to_clip(pos: vec3<f32>) -> vec4<f32> {
    return view_projection * (world * vec4<f32>(pos, 1.0));
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) layer: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) layer: u32,
};

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    // let world_pos = world * vec4<f32>(in.position, 1.0);
    // let view_pos = view * world_pos;
    // out.clip_position = projection * view_pos;
    out.clip_position = local_to_clip(in.position);
    out.uv = in.uv;
    out.layer = in.layer;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(array_texture, array_texture_sampler, in.uv, in.layer);
}