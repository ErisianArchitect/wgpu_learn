

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
    let direction = normalize(world_pos - camera_position);
    // let view_pos = view * world_pos;
    // out.clip_position = projection * view_pos;
    // out.world_pos = in.position + camera_position;
    out.clip_position = local_to_clip(in.position);
    out.direction = direction;
    return out;
}

// const Z_NEAR: f32 = 0.01;
// const Z_FAR: f32 = 1000.0;

fn fix_seams(direction: vec3<f32>) -> vec3<f32> {
    let m = max(max(abs(direction.x),abs(direction.y)), abs(direction.z));
    return direction / m;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // let view_distance = length(in.world_pos - camera_position);
    // let interp = saturate((view_distance - NEAR_DISTANCE) / (FAR_DISTANCE - NEAR_DISTANCE));
    // let near_sample = textureSample(array_texture, array_texture_near_sampler, in.uv, in.layer);
    var dir = fix_seams(in.direction);
    dir.y = dir.y * 5.0;
    let far_sample = textureSample(cubemap, cubemap_sampler, dir);
    return far_sample;
    // let sample = mix(near_sample, far_sample, interp);
    // if view_distance >= fog.start {
    //     // if view_distance > FOG_END {
    //     //     discard;
    //     // }
    //     let fog_interp = saturate((view_distance - fog.start) / (fog.end - fog.start));
    //     let fog_color = vec4<f32>(fog.color[0], fog.color[1], fog.color[2], fog.color[3]);
    //     return mix(sample, fog_color, smoothstep(0.0, 1.0, fog_interp));
    // } else {
    //     return sample;
    // }
}