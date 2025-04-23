use std::time::Duration;

pub mod state;
pub mod model;
pub mod voxel;
pub mod camera;
pub mod rendering;
pub mod math;
pub mod input;
pub mod framepace;
pub mod modeling;
pub mod gridzmo;
pub mod voxel_fog;
// pub mod text;
pub mod animation;
pub mod livemouse;
pub mod gizmo;
pub mod timing;
// mod trie;

pub struct FrameInfo {
    pub index: u64,
    pub fps: f64,
    pub last_frame_time: Duration,
    pub delta_time: Duration,
}