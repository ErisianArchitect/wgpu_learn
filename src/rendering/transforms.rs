pub struct TransformsBindGroup {
    // pub world_buffer: wgpu::Buffer,
    pub view_projection_buffer: wgpu::Buffer,
    pub camera_position_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl TransformsBindGroup {
    pub fn new(device: &wgpu::Device) -> Self {
        let view_projection_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("View and Projection Matrix Buffer"),
            size: std::mem::size_of::<glam::Mat4>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_position_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera Position Buffer"),
            size: std::mem::size_of::<glam::Vec3>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Transforms Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Transforms Bind Group"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: view_projection_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: camera_position_buffer.as_entire_binding(),
                },
            ],
        });
        Self {
            view_projection_buffer,
            camera_position_buffer,
            bind_group,
            bind_group_layout: layout,
        }
    }

    pub fn write_view_projection(&self, queue: &wgpu::Queue, view_projection: &glam::Mat4) {
        queue.write_buffer(
            &self.view_projection_buffer,
            0,
            bytemuck::bytes_of(view_projection),
        );
    }

    pub fn write_camera_position(&self, queue: &wgpu::Queue, camera_position: &glam::Vec3) {
        queue.write_buffer(
            &self.camera_position_buffer,
            0,
            bytemuck::bytes_of(camera_position),
        );
    }
}

