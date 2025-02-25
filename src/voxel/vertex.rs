
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    uv: [f32; 2],
    texindex: u32,
}

pub const fn vert(position: [f32; 3], uv: [f32; 2], texindex: u32) -> Vertex {
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
        vert([-0.5, 0.0, -0.5], [0.0, 0.0], 0), vert([0.5, 0.0, -0.5], [1.0, 0.0], 0),
        vert([-0.5, 0.0, 0.5], [0.0, 1.0], 0), vert([0.5, 0.0, 0.5], [1.0, 1.0], 0),
    ]; 

    pub const PLANE_INDICES: &'static [u16] = &[
        0, 1, 2,
        2, 1, 3,
    ];

    pub const fn new(position: [f32; 3], uv: [f32; 2], texindex: u32) -> Self {
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
}

#[test]
fn glam_test() {
    // glam::Mat4::look_to_rh()
}