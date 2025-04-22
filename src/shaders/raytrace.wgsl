// 64x64x64 = 262144
// 1mib

@group(0) @binding(0) var raycast_result: texture_storage_2d<rgba8unorm, write>;
@group(1) @binding(0) var<storage, read> voxel_chunk: array<u32>;
@group(2) @binding(0) var<uniform> camera: Camera;
@group(3) @binding(0) var directions: texture_storage_2d<rgba32float, read>;

// struct DirectionalLight {
//     inverse_direction: vec3<f32>,
//     color: vec3<f32>,
//     intensity: f32,
//     shadow: f32,
//     active: bool,
// }

struct Lighting {
    light_ray_direction: vec3<f32>,
    // 4 pad bytes
    
}

const LIGHTDIR: vec3<f32> = vec3<f32>(1.0, -2.0, 5.0);
// const LIGHTDIR: vec3<f32> = vec3<f32>(0.0, -1.0, 0.0);
const INVLIGHTDIR: vec3<f32> = -LIGHTDIR;
const SCREENSIZE: vec2<u32> = vec2<u32>(1920, 1080);

fn detect_edge(hit_fract: vec2<f32>) -> bool {
    const EDGE_WIDTH: f32 = 1.0 / 32.0;
    const MIN_EDGE: vec2<f32> = vec2<f32>(EDGE_WIDTH);
    const MAX_EDGE: vec2<f32> = vec2<f32>(1.0 - (EDGE_WIDTH));
    return any(hit_fract < MIN_EDGE | hit_fract >= MAX_EDGE);
}

const SMIDGEN: vec3<f32> = vec3<f32>(1e-4);
const UNSMIDGEN: vec3<f32> = vec3<f32>(1.0 - 1e-4);

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // (n << 11) == (n * 2048)
    // let index = (y << 11) + x;
    if any(global_id.xy > SCREENSIZE) {
        return;
    }
    textureStore(raycast_result, global_id.xy, trace_color(global_id.xy));
}

fn trace_color(texel: vec2<u32>) -> vec4<f32> {
    // let tx = i32(texel.x);
    // let ty = i32(texel.y);
    // let sx = i32(SCREENSIZE.x);
    // let sy = i32(SCREENSIZE.y);
    // let fx = (tx + sx / 2) % sx;
    // let fy = (ty + sy / 2) % sy;
    // let ray = get_ray(vec2<u32>(
    //     u32(fx),
    //     u32(fy),
    // ));
    let ray = get_ray(texel);
    let hit = raycast(ray, camera.near, camera.far);
    var color = vec3<f32>(0.0, 0.0, 0.0);
    var alpha = 0.0;
    if hit.hit {
        var hit_point = ray.pos + ray.dir * hit.distance;
        var face_fract = vec2<f32>(0.0);
        alpha = 1.0;
        var neighbor = hit.coord;
        switch hit.face {
            case NoFace: {
                alpha = 0.0;
                color = vec3<f32>(1.0, 1.0, 1.0);
            }
            case PosX: {
                neighbor.x += 1;
                let neighbor_cell = vec3<f32>(neighbor);
                hit_point = clamp(hit_point, neighbor_cell + SMIDGEN, neighbor_cell + UNSMIDGEN);
                face_fract = fract(hit_point.yz);
                color = vec3<f32>(1.0, 0.0, 0.0);
            }
            case PosY: {
                neighbor.y += 1;
                let neighbor_cell = vec3<f32>(neighbor);
                hit_point = clamp(hit_point, neighbor_cell + SMIDGEN, neighbor_cell + UNSMIDGEN);
                face_fract = fract(hit_point.xz);
                color = vec3<f32>(0.0, 1.0, 0.0);
            }
            case PosZ: {
                neighbor.z += 1;
                let neighbor_cell = vec3<f32>(neighbor);
                hit_point = clamp(hit_point, neighbor_cell + SMIDGEN, neighbor_cell + UNSMIDGEN);
                face_fract = fract(hit_point.xy);
                color = vec3<f32>(0.0, 0.0, 1.0);
            }
            case NegX: {
                neighbor.x -= 1;
                let neighbor_cell = vec3<f32>(neighbor);
                hit_point = clamp(hit_point, neighbor_cell + SMIDGEN, neighbor_cell + UNSMIDGEN);
                face_fract = fract(hit_point.yz);
                color = vec3<f32>(1.0, 1.0, 0.0);
            }
            case NegY: {
                neighbor.y -= 1;
                let neighbor_cell = vec3<f32>(neighbor);
                hit_point = clamp(hit_point, neighbor_cell + SMIDGEN, neighbor_cell + UNSMIDGEN);
                face_fract = fract(hit_point.xz);
                color = vec3<f32>(0.0, 1.0, 1.0);
            }
            case NegZ: {
                neighbor.z -= 1;
                let neighbor_cell = vec3<f32>(neighbor);
                hit_point = clamp(hit_point, neighbor_cell + SMIDGEN, neighbor_cell + UNSMIDGEN);
                face_fract = fract(hit_point.xy);
                color = vec3<f32>(1.0, 0.0, 1.0);
            }
            default: {}
        }
        let checker = ((hit.coord.x ^ hit.coord.y ^ hit.coord.z) & 1) != 0;
        if checker {
            color *= 0.3;
        }
        if detect_edge(face_fract) {
            color *= 0.1;
        }
        let light_ray = Ray(hit_point, normalize(INVLIGHTDIR));
        let light_hit = raycast(light_ray, 0.0, 112.0);
        if light_hit.hit {
            color *= 0.02;
        }
    }
    return vec4<f32>(color, alpha);
}

struct Camera {
    rotation: mat3x3<f32>,
    position: vec3<f32>,
    dimensions: vec2<u32>,
    near: f32,
    far: f32,
}

struct RayHit {
    coord: vec3<i32>,
    distance: f32,
    id: u32,
    face: u32,
    hit: bool,
}

struct U64 {
    low: u32,
    high: u32,
}

struct Ray {
    pos: vec3<f32>,
    dir: vec3<f32>,
}

// fn get_bit(value: U64, index: u32) -> bool {
//     let low = value.low & (u32(1) << index);
//     let high = value.high & (u32(1) << (index ^ u32(32)));
//     return (low | high) != u32(0);
// }

fn get_dir(coord: vec2<u32>) -> vec3<f32> {
    return textureLoad(directions, coord).xyz;
}

fn get_ray(coord: vec2<u32>) -> Ray {
    let dir = get_dir(coord);
    let pos = camera.position;
    return Ray(pos, rotate_dir(dir));
}

fn rotate_dir(dir: vec3<f32>) -> vec3<f32> {
    return camera.rotation * dir;
}

const NoFace: u32 = 0;
const NegX: u32 = 1;
const NegY: u32 = 2;
const NegZ: u32 = 3;
const PosX: u32 = 4;
const PosY: u32 = 5;
const PosZ: u32 = 6;

const MINPOS: f32 = 1.175494351e-38;
const F32MAX: f32 = 3.4028235e+38;
const NEGF32MAX: f32 = -F32MAX;
// The amount to add or remove to ray to either penetrate or not penetrate a voxel.
const RAY_PENETRATE: f32 = 1e-5;

fn get_block(coord: vec3<i32>) -> u32 {
    let xyz = coord.x | coord.y | coord.z;
    let uxyz = u32(xyz);
    if uxyz >= 64u {
        return 0u;
    }
    let index = u32(coord.y * 4096 + coord.z * 64 + coord.x);
    let id = voxel_chunk[index];
    return id;
}

fn calc_delta(mag: f32) -> f32 {
    return 1.0 / max(abs(mag), MINPOS);
}

fn calc_t_max(step: i32, fract: f32, mag: f32) -> f32 {
    if step > 0 {
        return (1.0 - fract) / max(abs(mag), MINPOS);
    } else if step < 0 {
        return fract / max(abs(mag), MINPOS);
    } else {
        return F32MAX;
    }
}

const ZERO: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);
const SIXTYFOUR: vec3<f32> = vec3<f32>(64.0, 64.0, 64.0);
const IZERO: vec3<i32> = vec3<i32>(0, 0, 0);
const NEGFACE: vec3<u32> = vec3<u32>(NegX, NegY, NegZ);
const POSFACE: vec3<u32> = vec3<u32>(PosX, PosY, PosZ);

fn raycast(ray: Ray, near: f32, far: f32) -> RayHit {
    // No hit:
    // RayHit(
    //     vec3<i32>(0, 0, 0), // coord
    //     0.0, // distance
    //     0, // id
    //     NoFace, // face
    //     false, // hit
    // );
    var pos = ray.pos;
    let dir = ray.dir;
    var delta_min = vec3<f32>(NEGF32MAX);
    var delta_max = vec3<f32>(F32MAX);
    var t_max_add = 0.0;
    var dir_sign = sign(dir);
    var step = vec3<i32>(dir_sign);
    let face = select(NEGFACE, POSFACE, step < IZERO);
    var enter_face = NoFace;

    let lt = pos < ZERO;
    let gt = pos >= SIXTYFOUR;
    let ltgt = lt | gt;
    if any(ltgt) {
        switch step.x + 1 {
            case 0: {
                if lt.x {
                    return RayHit(
                        vec3<i32>(0, 0, 0),
                        0.0,
                        0,
                        NoFace,
                        false,
                    );
                }
                delta_min.x = (pos.x - 64.0) / -dir.x;
                delta_max.x = pos.x / -dir.x;
            }
            case 1: {}
            case 2: {
                if gt.x {
                    return RayHit(
                        vec3<i32>(0, 0, 0),
                        0.0,
                        0,
                        NoFace,
                        false,
                    );
                }
                delta_min.x = -pos.x / dir.x;
                delta_max.x = (64.0 - pos.x) / dir.x;
            }
            default: {}
        }

        switch step.y + 1 {
            case 0: {
                if lt.y {
                    return RayHit(
                        vec3<i32>(0, 0, 0),
                        0.0,
                        0,
                        NoFace,
                        false,
                    );
                }
                delta_min.y = (pos.y - 64.0) / -dir.y;
                delta_max.y = pos.y / -dir.y;
            }
            case 1:{}
            case 2: {
                if gt.y {
                    return RayHit(
                        vec3<i32>(0, 0, 0),
                        0.0,
                        0,
                        NoFace,
                        false,
                    );
                }
                delta_min.y = -pos.y / dir.y;
                delta_max.y = (64.0 - pos.y) / dir.y;
            }
            default: {}
        }

        switch step.z + 1 {
            case 0: {
                if lt.z {
                    return RayHit(
                        vec3<i32>(0, 0, 0),
                        0.0,
                        0,
                        NoFace,
                        false,
                    );
                }
                delta_min.z = (pos.z - 64.0) / -dir.z;
                delta_max.z = pos.z / -dir.z;
            }
            case 1:{}
            case 2: {
                if gt.z {
                    return RayHit(
                        vec3<i32>(0, 0, 0),
                        0.0,
                        0,
                        NoFace,
                        false,
                    );
                }
                delta_min.z = -pos.z / dir.z;
                delta_max.z = (64.0 - pos.z) / dir.z;
            }
            default: {}
        }
        if delta_min.x > delta_min.y {
            if delta_min.x > delta_min.z {
                enter_face = face.x;
            } else {
                enter_face = face.z;
            }
        } else {
            if delta_min.y > delta_min.z {
                enter_face = face.y;
            } else {
                enter_face = face.z;
            }
        }
        t_max_add = max(delta_min.x, max(delta_min.y, delta_min.z)) + RAY_PENETRATE;
        let min_max = min(delta_max.x, max(delta_max.y, delta_max.z));
        if t_max_add >= min(min_max, far) {
            return RayHit(
                vec3<i32>(0, 0, 0),
                0.0,
                0,
                NoFace,
                false,
            );
        }
        pos = pos + dir * t_max_add;
    } else {
        switch step.x + 1 {
            case 0: {
                delta_max.x = pos.x / -dir.x;
            }
            case 1:{}
            case 2: {
                delta_max.x = (64.0 - pos.x) / dir.x;
            }
            default: {}
        }

        switch step.y + 1 {
            case 0: {
                delta_max.y = pos.y / -dir.y;
            }
            case 1:{}
            case 2: {
                delta_max.y = (64.0 - pos.y) / dir.y;
            }
            default: {}
        }

        switch step.z + 1 {
            case 0: {
                delta_max.z = pos.z / -dir.z;
            }
            case 1:{}
            case 2: {
                delta_max.z = (64.0 - pos.z) / dir.z;
            }
            default: {}
        }
    }
    // let face = vec3<u32>(
    //     select(step.x < 0.0, NEGFACE.x, POSFACE.x),
    //     select(step.y < 0.0, NEGFACE.y, POSFACE.y),
    //     select(step.z < 0.0, NEGFACE.z, POSFACE.z),
    // );

    var cell = vec3<i32>(floor(pos));
    let hit_id = get_block(cell);
    if hit_id != 0 {
        var hit_face = enter_face;
        // if t_max_add == delta_min.x {
        //     hit_face = face.x;
        // } else if t_max_add == delta_min.y {
        //     hit_face = face.y;
        // } else if t_max_add == delta_min.z {
        //     hit_face = face.z;
        // }
        return RayHit(
            cell,
            t_max_add,
            hit_id,
            hit_face,
            true,
        );
    }
    let delta = vec3<f32>(
        calc_delta(dir.x),
        calc_delta(dir.y),
        calc_delta(dir.z),
    );
    let fract = fract(pos);
    var t_max = vec3<f32>(
        calc_t_max(step.x, fract.x, dir.x) + t_max_add,
        calc_t_max(step.y, fract.y, dir.y) + t_max_add,
        calc_t_max(step.z, fract.z, dir.z) + t_max_add,
    );
    let max_dist = vec3<f32>(
        min(delta_max.x, far),
        min(delta_max.y, far),
        min(delta_max.z, far),
    );
    loop {
        if t_max.x <= t_max.y {
            if t_max.x <= t_max.z {
                if t_max.x >= max_dist.x {
                    return RayHit(
                        vec3<i32>(0, 0, 0),
                        0.0,
                        0,
                        NoFace,
                        false,
                    );
                }
                cell.x = cell.x + step.x;
                let hit_id = get_block(cell);
                if hit_id != 0 {
                    return RayHit(
                        cell,
                        t_max.x,
                        hit_id,
                        face.x,
                        true,
                    );
                }
                t_max.x = t_max.x + delta.x;
            } else {
                if t_max.z >= max_dist.z {
                    return RayHit(
                        vec3<i32>(0, 0, 0),
                        0.0,
                        0,
                        NoFace,
                        false,
                    );
                }
                cell.z = cell.z + step.z;
                let hit_id = get_block(cell);
                if hit_id != 0 {
                    return RayHit(
                        cell,
                        t_max.z,
                        hit_id,
                        face.z,
                        true,
                    );
                }
                t_max.z = t_max.z + delta.z;
            }
        } else {
            if t_max.y <= t_max.z {
                if t_max.y >= max_dist.y {
                    return RayHit(
                        vec3<i32>(0, 0, 0),
                        0.0,
                        0,
                        NoFace,
                        false,
                    );
                }
                cell.y = cell.y + step.y;
                let hit_id = get_block(cell);
                if hit_id != 0 {
                    return RayHit(
                        cell,
                        t_max.y,
                        hit_id,
                        face.y,
                        true,
                    );
                }
                t_max.y = t_max.y + delta.y;
            } else {
                if t_max.z >= max_dist.z {
                    return RayHit(
                        vec3<i32>(0, 0, 0),
                        0.0,
                        0,
                        NoFace,
                        false,
                    );
                }
                cell.z = cell.z + step.z;
                let hit_id = get_block(cell);
                if hit_id != 0 {
                    return RayHit(
                        cell,
                        t_max.z,
                        hit_id,
                        face.z,
                        true,
                    );
                }
                t_max.z = t_max.z + delta.z;
            }
        }
    }
    return RayHit(
        vec3<i32>(0, 0, 0),
        0.0,
        0,
        NoFace,
        false,
    );
}