@group(0) @binding(0)
var render_texture: texture_2d<f32>;
@group(0) @binding(1)
var render_texture_sampler: sampler;

struct Vertex {
    pos: vec2<f32>,
    uv: vec2<f32>,
}

const VERTICES: array<Vertex, 4> = array<Vertex, 4>(
    Vertex(vec2<f32>(-1.0, 1.0), vec2<f32>(0.0, 0.0)),  Vertex(vec2<f32>(1.0, 1.0), vec2<f32>(1.0, 0.0)),
    Vertex(vec2<f32>(-1.0, -1.0), vec2<f32>(0.0, 1.0)), Vertex(vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0)),
);

const INDICES: array<u32, 6> = array<u32, 6>(
    0, 2, 1, 1, 2, 3,
);

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vertex_main(
    @builtin(vertex_index) vi: u32
) -> VertexOutput {
    let vertex = VERTICES[INDICES[vi]];
    var out: VertexOutput;
    out.clip_position = vec4<f32>(vertex.pos, 0.0, 1.0);
    out.uv = vertex.uv;
    return out;
}

@fragment
fn fragment_main(
    in: VertexOutput,
) -> @location(0) vec4<f32> {
    return textureSample(render_texture, render_texture_sampler, in.uv);
}