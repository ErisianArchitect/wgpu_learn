// transform
// offset_x, offset_y
// width, height, sharp_line_width, line_width
// edge_color, inner_edge_color, sharp_line_color

// mat4x4<f32>
// f32, f32
// f32, f32, f32, f32
// vec4<u8>, vec4<u8>, vec4<u8>

struct PushData {
    transform: mat4x4<f32>,
    offset: vec2<f32>,
    size: vec2<f32>,
    // sharp_line_width should be less than or equal to line_width
    // 
    sharp_line_width: f32,
    line_width: f32,
}
var<push_constant> push: PushData;

const VERTICES: []