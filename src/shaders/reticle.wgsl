@group(0) @binding(0) var<uniform> ortho_matrix: mat4x4<f32>;
@group(0) @binding(1) var reticle_texture: texture_2d<f32>;
@group(0) @binding(2) var reticle_sampler: sampler;
@group(0) @binding(3) var<uniform> dimensions: vec2<f32>;

struct Vertex {
    pos: vec2<f32>,
    uv: vec2<f32>,
}

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

const VERTICES: array<Vertex, 4> = array<Vertex, 4>(
    // Top-Left
    Vertex(vec2<f32>(-36.0, -36), vec2<f32>(0.0, 0.0)),
    // Top-Right
    Vertex(vec2<f32>(36.0, -36), vec2<f32>(1.0, 0.0)),
    // Bottom-Left
    Vertex(vec2<f32>(-36.0, 36), vec2<f32>(0.0, 1.0)),
    // Bottom-Right
    Vertex(vec2<f32>(36.0, 36), vec2<f32>(1.0, 1.0)),
);

const INDICES: array<u32, 6> = array<u32, 6>(
    0, 2, 1, 1, 2, 3,
);

@vertex
fn vs_main(
    @builtin(vertex_index) index: u32,
) -> VertexOut {
    var out: VertexOut;
    let vertex = VERTICES[INDICES[index]];
    let pos = ortho_matrix * vec4<f32>(vertex.pos + dimensions / 2.0, 0.0, 1.0);
    out.clip_position = pos;
    out.uv = vertex.uv;
    return out;
}

@fragment
fn fs_main(
    in: VertexOut
) -> @location(0) vec4<f32> {
    let sample = textureSample(reticle_texture, reticle_sampler, in.uv);
    return sample;
}