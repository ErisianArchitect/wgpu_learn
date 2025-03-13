
fn quadratic(t: f32) -> f32 {
    return pow(t, 2.0);
}

fn cubic(t: f32) -> f32 {
    return pow(t, 3.0);
}

fn quartic(t: f32) -> f32 {
    return pow(t, 4.0);
}

fn quintic(t: f32) -> f32 {
    return pow(t, 5.0);
}

const EASING_PI: f32 = 3.141592653589793;
const EASING_FRAC_PI_2: f32 = EASING_PI / 2.0;

fn sine_in(t: f32) -> f32 {
    return 1.0 - cos(t * EASING_FRAC_PI_2);
}

fn sine_out(t: f32) -> f32 {
    return sin(t * EASING_FRAC_PI_2);
}

fn sine_in_out(t: f32) -> f32 {
    return 0.5 * (1.0 - cos(EASING_PI * t));
}

fn circular_in(t: f32) -> f32 {
    return 1.0 - sqrt(1.0 - pow(t, 2.0));
}

fn circular_out(t: f32) -> f32 {
    return sqrt(1.0 - pow(1.0 - t, 2.0));
}

fn circular_in_out(t: f32) -> f32 {
    if t < 0.5 {
        return 0.5 * (1.0 - sqrt(1.0 - (4.0 * pow(t, 2.0))));
    } else {
        return 0.5 * (sqrt(1.0 - 4.0 * pow(1.0 - t, 2.0)) + 1.0);
    }
}

struct Fog {
    color: vec4<f32>,
    start: f32,
    end: f32,
}

// @group(0) @binding(0) var<uniform> world: mat4x4<f32>;
var<push_constant> world: mat4x4<f32>;
@group(0) @binding(0) var<uniform> view_projection: mat4x4<f32>;
@group(0) @binding(1) var<uniform> camera_position: vec3<f32>;

@group(1) @binding(0) var array_texture: texture_2d_array<f32>;
@group(1) @binding(1) var array_texture_sampler: sampler;
// @group(1) @binding(2) var array_texture_far_sampler: sampler;

@group(2) @binding(0) var<uniform> fog: Fog;

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
    @location(0) uv: vec2<f32>,
    @location(1) layer: u32,
    @location(2) world_pos: vec3<f32>,
};

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    // let world_pos = world * vec4<f32>(in.position, 1.0);
    // let view_pos = view * world_pos;
    // out.clip_position = projection * view_pos;
    out.world_pos = local_to_world(in.position);
    out.clip_position = local_to_clip(in.position);
    out.uv = in.uv;
    out.layer = in.layer;
    return out;
}

// const Z_NEAR: f32 = 0.01;
// const Z_FAR: f32 = 1000.0;
const NEAR_DISTANCE: f32 = 16.0;
const FAR_DISTANCE: f32 = 40.0;
const FOG_START: f32 = 10.0;
const FOG_END: f32 = 180.0;
const GRAY: f32 = 0.0;
const GRAY_RGBA: vec4<f32> = vec4<f32>(GRAY, GRAY, GRAY, 1.0);

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let sample = textureSample(array_texture, array_texture_sampler, in.uv, in.layer);
    let view_distance = length(in.world_pos - camera_position);
    if view_distance >= fog.start {
        if fog.color.a <= 0.00001 && view_distance >= FOG_END {
            discard;
        }
        let fog_interp = saturate((view_distance - fog.start) / (fog.end - fog.start));
        return mix(sample, fog.color, smoothstep(0.0, 1.0, circular_in(fog_interp)));
    } else {
        return sample;
    }
    // let interp = circular_out(saturate((view_distance - NEAR_DISTANCE) / (FAR_DISTANCE - NEAR_DISTANCE)));
    // let far_sample = textureSample(array_texture, array_texture_far_sampler, in.uv, in.layer);
    // let sample = mix(near_sample, far_sample, interp);
    
}