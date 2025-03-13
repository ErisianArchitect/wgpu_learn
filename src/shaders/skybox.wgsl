

var<push_constant> inv_view_proj: mat4x4<f32>;

@group(0) @binding(0) var cubemap: texture_cube<f32>;
@group(0) @binding(1) var cubemap_sampler: sampler;

// fn local_to_clip(pos: vec3<f32>) -> vec4<f32> {
//     return view_projection * (world * vec4<f32>(pos, 1.0));
// }

// fn local_to_world(pos: vec3<f32>) -> vec3<f32> {
//     return (world * vec4<f32>(pos, 1.0)).xyz;
// }

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(1) direction: vec3<f32>,
};

// const vertices: array<vec3<f32>, 8> = array<vec3<f32>, 8>(
//     //    Top
//     //    L R
//     //  F 0 1
//     //  B 2 3
//     vec2<f32>(-1.0, 1.0, -1.0), vec2<f32>(1.0, 1.0, -1.0),
//     vec2<f32>(-1.0, 1.0, 1.0), vec2<f32>(1.0, 1.0, 1.0),
//     // Bottom
//     //    L R
//     //  F 4 5
//     //  B 6 7
//     vec2<f32>(-1.0, -1.0, -1.0), vec2<f32>(1.0, -1.0, -1.0),
//     vec2<f32>(-1.0, -1.0, 1.0), vec2<f32>(1.0, -1.0, 1.0),
// );

// const indices: array<u32, 36> = array<u32, 36>(
//     2, 0, 1, 2, 1, 3, // Top
//     4, 6, 5, 6, 7, 5, // Bottom
//     0, 2, 6, 0, 6, 4, // Left
//     1, 5, 3, 3, 5, 7, // Right
//     0, 4, 1, 1, 4, 5, // Front
//     3, 7, 2, 2, 7, 6, // Back
// );

const vertices: array<vec2<f32>, 4> = array<vec2<f32>, 4>(
    vec2<f32>(-1.0, 1.0), vec2<f32>(1.0, 1.0),
    vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0),
);

const indices: array<u32, 6> = array<u32, 6>(
    0, 2, 1,
    1, 2, 3,
);

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let vert = vertices[indices[vertex_index]];
    out.position = vec4<f32>(vert.x, vert.y, 1.0, 1.0);
    out.direction = (inv_view_proj * out.position).xyz;
    return out;
}

fn fix_seams(direction: vec3<f32>) -> vec3<f32> {
    let m = max(max(abs(direction.x),abs(direction.y)), abs(direction.z));
    return direction / m;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(cubemap, cubemap_sampler, fix_seams(in.direction));
}