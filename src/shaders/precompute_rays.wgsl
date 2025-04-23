@group(0) @binding(0) var directions: texture_storage_2d<rgba32float, write>;
@group(0) @binding(1) var<uniform> ndc_mult: vec2<f32>;

const U32MAX: u32 = 4294967295;

const SCREENSIZE: vec2<u32> = vec2<u32>(1920, 1080);
const DIMENSIONS: vec2<f32> = vec2<f32>(SCREENSIZE);
const HALF2: vec2<f32> = vec2<f32>(0.5, 0.5);

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // y is inverted here, and ndc_mult.y is negative, so when
    // ndc is multiplied by ndc_mult, we get the correct coordinate.
    if any(global_id.xy > SCREENSIZE) {
        return;
    }
    var ndc = ((vec2<f32>(global_id.xy) + HALF2) / DIMENSIONS) * 2.0 - 1.0;
    let xy = ndc * ndc_mult;
    let dir = normalize(vec3<f32>(xy, -1.0));
    let store = vec4<f32>(dir, 0.0);
    textureStore(directions, global_id.xy, store);
}