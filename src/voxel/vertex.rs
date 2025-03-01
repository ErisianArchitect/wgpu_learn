use glam::*;

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: glam::Vec3,
    pub uv: glam::Vec2,
    pub texindex: u32,
}

pub const fn pos(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

pub const fn uv(x: f32, y: f32) -> Vec2 {
    Vec2::new(x, y)
}

pub const fn index(i: u32) -> u32 {
    i
}

pub const fn vert(position: glam::Vec3, uv: glam::Vec2, texindex: u32) -> Vertex {
    Vertex {
        position,
        uv,
        texindex,
    }
}

impl Vertex {
    pub const ATTRIBS: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x2,
        2 => Uint32
    ];

    pub const PLANE_VERTICES: &'static [Self] = &[
        vert(pos(-0.5, 0.0, -0.5), uv(0.0, 0.0), index(4)), vert(pos(0.5, 0.0, -0.5), uv(1.0, 0.0), index(4)),
        vert(pos(-0.5, 0.0, 0.5), uv(0.0, 1.0), index(4)), vert(pos(0.5, 0.0, 0.5), uv(1.0, 1.0), index(4)),
    ]; 

    pub const PLANE_INDICES: &'static [u16] = &[
        0, 2, 1,
        2, 3, 1,
    ];

    pub const fn new(position: Vec3, uv: Vec2, texindex: u32) -> Self {
        Self {
            position,
            uv,
            texindex,
        }
    }

    pub const fn vertex_buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBS,
        }
    }

    pub const fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBS,
        }
    }
}

#[test]
fn glam_test() {
    // glam::Mat4::look_to_rh()
}