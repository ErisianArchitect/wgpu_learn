@group(0) @binding(0) var directions: texture_storage_2d<rgba32float, write>;
@group(0) @binding(1) var<uniform> ndc_mult: vec2<f32>;

const U32MAX: u32 = 4294967295;

@compute @workgroup_size(16, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let ndc = (vec2<f32>(global_id.xy) / vec2<f32>(dimensions)) * 2.0 - 1.0;
    let xy = ndc * ndc_mult;
    let dir = normalize(vec3<f32>(xy, -1.0));
    let store = vec4<f32>(dir, 0.0);
    textureStore(directions, global_id.xy, store);
}