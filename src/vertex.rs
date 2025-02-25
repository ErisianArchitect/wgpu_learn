#[repr(C)]
#[derive(Debug, Default, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

// const VERTICES: &[Vertex] = &[
//     Vertex { position: [0.0, 0.5, 0.0], color: [1.0, 0.0, 0.0] },
//     Vertex { position: [-0.5, -0.5, 0.0], color: [0.0, 1.0, 0.0] },
//     Vertex { position: [0.5, -0.5, 0.0], color: [0.0, 0.0, 1.0] },
// ];

pub const fn vert(pos: [f32; 3], color: [f32; 3]) -> Vertex {
    Vertex::new(pos, color)
}

impl Vertex {
    // I'm going to leave this here so I can see how to do it without the macro.
    // &[
    //     wgpu::VertexAttribute {
    //         offset: 0,
    //         shader_location: 0,
    //         format: wgpu::VertexFormat::Float32x3,
    //     },
    //     wgpu::VertexAttribute {
    //         offset: std::mem::offset_of!(Vertex, color) as wgpu::BufferAddress,
    //         shader_location: 1,
    //         format: wgpu::VertexFormat::Float32x3,
    //     }
    // ];
    const ATTRIBS: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
    // pub const TRIANGLE: &'static [Self] = &[
    //     vert([0.0, 0.5, 0.0], [1.0, 0.0, 0.0]),
    //     vert([-0.5, -0.5, 0.0], [0.0, 1.0, 0.0]),
    //     vert([0.5, -0.5, 0.0], [0.0, 0.0, 1.0]),
    // ];
    pub const PENTAGON: &'static [Self] = &[
        vert([-0.0868241, 0.49240386, 0.0], [0.5, 0.0, 0.5]),
        vert([-0.49513406, 0.06958647, 0.0], [0.5, 0.0, 0.5]),
        vert([-0.21918549, -0.44939706, 0.0], [0.5, 0.0, 0.5]),
        vert([0.35966998, -0.3473291, 0.0], [0.5, 0.0, 0.5]),
        vert([0.44147372, 0.2347359, 0.0], [0.5, 0.0, 0.5]),
    ];

    pub const PENTAGON_INDICES: &'static [u16] = &[
        0, 1, 4,
        1, 2, 4,
        2, 3, 4,
    ];

    pub const fn new(pos: [f32; 3], color: [f32; 3]) -> Self {
        Self {
            position: pos,
            color,
        }
    }

    pub const fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBS,
        }
    }
}