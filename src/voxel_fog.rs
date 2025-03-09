use bytemuck::NoUninit;
use glam::Vec4;


#[repr(C)]
#[repr(align(16))]
#[derive(Debug, Clone, Copy, NoUninit)]
pub struct Fog {
    pub color: [f32; 4],
    pub start: f32,
    pub end: f32,
    pub padding: [u8; 8],
}

impl Fog {
    pub fn new(start: f32, end: f32, color: Vec4) -> Self {
        Self {
            start,
            end,
            color: color.to_array(),
            padding: [0; 8],
        }
    }

    pub fn set_start(&mut self, start: f32) {
        self.start = start;
    }

    pub fn set_end(&mut self, end: f32) {
        self.end = end;
    }

    pub fn set_color(&mut self, color: Vec4) {
        self.color = color.to_array();
    }
}

pub struct FogBindGroup {
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl FogBindGroup {
    pub fn new(device: &wgpu::Device) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Fog Buffer"),
            size: std::mem::size_of::<Fog>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Fog Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fog Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                },
            ],
        });
        Self {
            buffer,
            bind_group,
            bind_group_layout,
        }
    }

    pub fn write_fog(&self, queue: &wgpu::Queue, fog: &Fog) {
        queue.write_buffer(
            &self.buffer,
            0,
            bytemuck::bytes_of(fog),
        );
    }
}