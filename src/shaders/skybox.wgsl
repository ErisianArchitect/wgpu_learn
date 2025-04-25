

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

struct U64 {
    low: u32,
    high: u32,
}

fn get_bit(value: U64, index: u32) -> bool {
    let low = value.low & (1u << index);
    let high = value.high & (1u << (index ^ 32u));
    return (low | high) != 0;
}

fn count_bits_before(value: U64, index: u32) -> u32 {
    let high_mask = extractBits(value.high, 0u, max(index, 32u) - 32u);
    let high_count = countOneBits(high_mask);
    let low_mask = extractBits(value.low, 0u, min(index, 32u));
    let low_count = countOneBits(low_mask);
    return low_count + high_count;
}

fn test_count_bits(value: U64, index: u32, expect: u32) -> bool {
    if count_bits_before(value, index) == expect {
        return true;
    } else {
        return false;
    }
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // var dir = fix_seams(in.direction);
    var dir = in.direction;
    // dir.y = dir.y * 5.0;
    // const V: f32 = 1.0 / 1e-18;
    // count_bits_before test.
    // const U64MAX: U64 = U64(0xffffffffu, 0xffffffffu);
    // let test0 = test_count_bits(U64MAX, 64, 64);
    // let test1 = test_count_bits(U64MAX, 63, 63);
    // let test2 = test_count_bits(U64MAX, 48, 48);
    // let test3 = test_count_bits(U64MAX, 32, 32);
    // let test4 = test_count_bits(U64MAX, 31, 31);
    // let test5 = test_count_bits(U64MAX, 16, 16);
    // let test6 = test_count_bits(U64MAX, 1, 1);
    // let test7 = test_count_bits(U64MAX, 0, 0);
    // if test0
    // && test1
    // && test2
    // && test3
    // && test4
    // && test5
    // && test6
    // && test7 {
    //     return vec4<f32>(0.0, 1.0, 0.0, 1.0);
    // } else {
    //     return vec4<f32>(1.0, 0.0, 0.0, 1.0);
    // }
    let far_sample = textureSample(cubemap, cubemap_sampler, dir);
    return far_sample;
}