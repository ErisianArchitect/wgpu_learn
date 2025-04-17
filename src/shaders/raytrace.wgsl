// 64x64x64 = 262144
// 1mib

@group(0) @binding(0) var raycast_result: texture_storage_2d<rgba8unorm, write>;
@group(1) @binding(0) var<storage, read> voxel_chunk: array<u32>;
@group(2) @binding(0) var<uniform> camera: Camera;
@group(3) @binding(0) var directions: texture_storage_2d<rgba32float, read>;


fn foo() {
    
}

@compute @workgroup_size(16, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // (n << 11) == (n * 2048)
    // let index = (y << 11) + x;
    let ray = get_ray(global_id.xy);
    let hit = raycast(ray);
    var color = vec3<f32>(0.0, 0.0, 0.0);
    if hit.hit {
        switch hit.face {
            case NoFace:
                color = vec3<f32>(1.0, 1.0, 1.0);
            break;
            case PosX:
                color = vec3<f32>(1.0, 0.0, 0.0);
            break;
            case PosY:
                color = vec3<f32>(0.0, 1.0, 0.0);
            break;
            case PosZ:
                color = vec3<f32>(0.0, 0.0, 1.0);
            break;
            case NegX:
                color = vec3<f32>(1.0, 1.0, 0.0);
            break;
            case NegY:
                color = vec3<f32>(0.0, 1.0, 1.0);
            break;
            case NegZ:
                color = vec3<f32>(1.0, 0.0, 1.0);
            break;
        }
    }
    textureStore(raycast_result, global_id.xy, vec4<f32>(color, 1.0));
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
    face: Face,
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
    return Ray(pos, dir);
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
        return INF;
    }
}

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
    var delta_min = vec3<f32>(NEG_INF);
    var delta_max = vec3<f32>(INF);
    var t_max_add = 0.0;
    var dir_sign = sign(dir);
    var step = vec3<i32>(dir_sign);
    var enter_face = NoFace;
    const ZERO: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);
    const SIXTYFOUR: vec3<f32> = vec3<f32>(64.0, 64.0, 64.0);
    if any(pos < ZERO) || any(pos >= SIXTYFOUR) {
        switch step.x + 1 {
            case 0:
                delta_min.x = (pos.x - 64.0) / -dir.x;
                delta_max.x = pos.x / -dir.x;
            break;
            case 1:break;
            case 2:
                delta_min.x = -pos.x / dir.x;
                delta_max.x = (64.0 - pos.x) / dir.x;
            break;
        }

        switch step.y + 1 {
            case 0:
                delta_min.y = (pos.y - 64.0) / -dir.y;
                delta_max.y = pos.y / -dir.y;
            break;
            case 1:break;
            case 2:
                delta_min.y = -pos.y / dir.y;
                delta_max.y = (64.0 - pos.y) / dir.y;
            break;
        }

        switch step.z + 1 {
            case 0:
                delta_min.z = (pos.z - 64.0) / -dir.z;
                delta_max.z = pos.z / -dir.z;
            break;
            case 1:break;
            case 2:
                delta_min.z = -pos.z / dir.z;
                delta_max.z = (64.0 - pos.z) / dir.z;
            break;
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
            case 0:
                delta_max.x = pos.x / -dir.x;
            break;
            case 1:break;
            case 2:
                delta_max.x = (64.0 - pos.x) / dir.x;
            break;
        }

        switch step.y + 1 {
            case 0:
                delta_max.y = pos.y / -dir.y;
            break;
            case 1:break;
            case 2:
                delta_max.y = (64.0 - pos.y) / dir.y;
            break;
        }

        switch step.z + 1 {
            case 0:
                delta_max.z = pos.z / -dir.z;
            break;
            case 1:break;
            case 2:
                delta_max.z = (64.0 - pos.z) / dir.z;
            break;
        }
    }

    const IZERO: vec3<i32> = vec3<i32>(0, 0, 0);
    const NEGFACE: vec3<u32> = vec3<u32>(NegX, NegY, NegZ);
    const POSFACE: vec3<u32> = vec3<u32>(PosX, PosY, PosZ);
    let face = select(step < IZERO, NEGFACE, POSFACE);

    var cell = vec3<i32>(floor(pos));
    let hit_id = get_block(cell);
    if hit_id != 0 {
        var hit_face = NoFace;
        if t_max_add == delta_min.x {
            hit_face = face.x;
        } else if t_max_add == delta_min.y {
            hit_face = face.y;
        } else if t_max_add == delta_min.z {
            hit_face = face.z;
        }
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
                cell.x += step.x;
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
                t_max.x += delta.x;
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
                cell.z += step.z;
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
                t_max.z += delta.z;
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
                cell.y += step.y;
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
                t_max.y += delta.y;
            } else {
                if t_max.z >= max_dist.z {
                    return RayHit(
                        vec3<i32>(0, 0, 0),
                        distance: 0.0,
                        0,
                        NoFace,
                        false,
                    );
                }
                cell.z += step.z;
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
                t_max.z += delta.z;
            }
        }
    }
}