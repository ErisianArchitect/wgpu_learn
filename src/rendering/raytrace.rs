use glam::*;
use bytemuck::{NoUninit, Pod, Zeroable};
use wgpu::util::{DeviceExt, RenderEncoder};
use crate::{camera::Camera, math::*};

#[repr(u32)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, NoUninit)]
pub enum Face {
    #[default]
    None = 0,
    PosX = 1,
    PosY = 2,
    PosZ = 3,
    NegX = 4,
    NegY = 5,
    NegZ = 6,
}

#[derive(Debug, Clone, Copy)]
pub struct RayCalc {
    mult: Vec2,
}

/// Calculates the value that you multiply the NDC coordinates by to get
/// the ray facing direction for those NDC coordinates with the given
/// field of view (`fov_rad`) and screen size. Screen size is the size
/// of the rendering area. So if you had a resolution of `1920x1080`,
/// `screen_size` would be `(1920, 1080)`.
#[inline]
pub fn calc_ray_mult(fov_rad: f32, screen_size: (u32, u32)) -> Vec2 {
    let aspect_ratio = screen_size.0 as f32 / screen_size.1 as f32;
    let tan_fov_half = (fov_rad * 0.5).tan();
    let asp_fov = aspect_ratio * tan_fov_half;
    vec2(asp_fov, -tan_fov_half)
}

#[inline]
pub const fn padding<const SIZE: usize>() -> [u8; SIZE] {
    [0u8; SIZE]
}

#[repr(C)]
#[derive(Debug, Clone, Copy, NoUninit)]
pub struct RayHit {
    pub coord: IVec3,
    _coord_pad: [u8; 4],
    pub distance: f32,
    pub id: u32,
    pub face: Face,
    pub hit: bool,
    _hit_pad: [u8; 3],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, NoUninit)]
pub struct GpuMat3 {
    pub mat: [GpuVec3; 3],
}

impl GpuMat3 {
    #[inline]
    pub fn new(mat: Mat3) -> Self {
        Self {
            mat: mat.to_cols_array_2d().map(|col| GpuVec3::new(col[0], col[1], col[2])),
        }
    }

    pub fn set(&mut self, mat: Mat3) {
        self.mat = mat.to_cols_array_2d().map(|col| GpuVec3::new(col[0], col[1], col[2]));
    }
}

impl From<Mat3> for GpuMat3 {
    fn from(value: Mat3) -> Self {
        Self::new(value)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuVec3 {
    pub vec: [f32; 3],
    _padding: [u8; 4],
}

impl From<Vec3> for GpuVec3 {
    fn from(value: Vec3) -> Self {
        Self::from_vec3(value)
    }
}

impl GpuVec3 {
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            vec: [x, y, z],
            _padding: padding(),
        }
    }

    #[inline]
    pub fn set(&mut self, vec: Vec3) {
        self.vec = vec.to_array()
    }

    #[inline]
    pub const fn from_vec3(vec: Vec3) -> Self {
        Self {
            vec: vec.to_array(),
            _padding: padding(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, NoUninit)]
pub struct Dim {
    pub width: u32,
    pub height: u32,
}

impl Dim {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, NoUninit)]
pub struct RenderRange {
    pub near: f32,
    pub far: f32,
}

impl RenderRange {
    pub const fn new(near: f32, far: f32) -> Self {
        Self { near, far }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, NoUninit)]
pub struct GpuTransform {
    pub rotation: GpuMat3,
    pub position: GpuVec3,
}

impl GpuTransform {
    pub const fn new(rotation: GpuMat3, position: GpuVec3) -> Self {
        Self { rotation, position }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, NoUninit)]
pub struct GpuRaytraceCamera {
    pub transform: GpuTransform,
    pub dimensions: Dim,
    pub range: RenderRange,
}

impl GpuRaytraceCamera {
    pub fn new(camera: &Camera, near: f32, far: f32) -> Self {
        let range = RenderRange::new(near, far);
        let transform = GpuTransform::new(
            GpuMat3::new(camera.rotation_matrix()),
            GpuVec3::from_vec3(camera.position),
        );
        Self {
            transform,
            dimensions: Dim::new(camera.screen_size.width, camera.screen_size.height),
            range,
        }
    }
}

pub struct RaytraceCamera {
    pub gpu_cam: GpuRaytraceCamera,
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl RaytraceCamera {
    pub fn new(camera: &Camera, device: &wgpu::Device) -> Self {
        let gpu_cam = GpuRaytraceCamera::new(camera, 0.1, 1000.0);
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Raytrace Camera Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::bytes_of(&gpu_cam),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Raytrace Camera Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                visibility: wgpu::ShaderStages::COMPUTE,
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Raytrace Camera Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }]
        });
        Self {
            gpu_cam,
            buffer,
            bind_group,
            bind_group_layout,
        }
    }

    pub fn bind(&self, index: u32, compute_pass: &mut wgpu::ComputePass) {
        compute_pass.set_bind_group(index, &self.bind_group, &[]);
    }

    pub fn write_transform(&mut self, transform: GpuTransform, queue: &wgpu::Queue) {
        self.gpu_cam.transform = transform;
        const TRANSFORM_SIZE: usize = std::mem::size_of::<GpuTransform>();
        const TRANSFORM_OFFSET: usize = std::mem::offset_of!(GpuRaytraceCamera, transform);
        const TRANSFORM_OFFSET_END: usize = TRANSFORM_OFFSET + TRANSFORM_SIZE;
        const TRANSFORM_RANGE: std::ops::Range<usize> = TRANSFORM_OFFSET..TRANSFORM_OFFSET_END;
        queue.write_buffer(
            &self.buffer,
            TRANSFORM_OFFSET as u64,
            &bytemuck::bytes_of(&self.gpu_cam)[TRANSFORM_RANGE],
        );
    }

    pub fn write_dimensions(&mut self, width: u32, height: u32, queue: &wgpu::Queue) {
        self.gpu_cam.dimensions = Dim::new(width, height);
        // I know it's 8 bytes, but I'd rather use size_of just in case.
        const DIM_SIZE: usize = std::mem::size_of::<Dim>();
        const DIM_OFFSET: usize = std::mem::offset_of!(GpuRaytraceCamera, dimensions);
        const DIM_OFFSET_END: usize = DIM_OFFSET + DIM_SIZE;
        const DIM_RANGE: std::ops::Range<usize> = DIM_OFFSET..DIM_OFFSET_END;
        queue.write_buffer(
            &self.buffer,
            DIM_OFFSET as u64,
            &bytemuck::bytes_of(&self.gpu_cam)[DIM_RANGE],
        );
    }

    pub fn write_range(&mut self, near: f32, far: f32, queue: &wgpu::Queue) {
        self.gpu_cam.range = RenderRange::new(near, far);
        const RANGE_SIZE: usize = std::mem::size_of::<RenderRange>();
        const RANGE_OFFSET: usize = std::mem::offset_of!(GpuRaytraceCamera, range);
        const RANGE_OFFSET_END: usize = RANGE_OFFSET + RANGE_SIZE;
        const RANGE_RANGE: std::ops::Range<usize> = RANGE_OFFSET..RANGE_OFFSET_END;
        queue.write_buffer(
            &self.buffer,
            RANGE_OFFSET as u64,
            &bytemuck::bytes_of(&self.gpu_cam)[RANGE_RANGE],
        );
    }

    /// This method should generally only be called once: when first setting
    /// the camera. You should otherwise use the specific field writers.
    pub fn write_camera(&mut self, camera: &Camera, queue: &wgpu::Queue) {
        self.gpu_cam.transform = GpuTransform::new(
            GpuMat3::new(camera.rotation_matrix()),
            GpuVec3::from_vec3(camera.position),
        );
        self.gpu_cam.dimensions = Dim::new(
            camera.screen_size.width,
            camera.screen_size.height
        );
        self.gpu_cam.range = RenderRange::new(camera.z_near, camera.z_far);
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&self.gpu_cam));
    }
}

pub struct PrecomputedDirections {
    // This never needs to be accessed CPU side.
    pub directions: wgpu::Texture,
    pub ndc_mult: wgpu::Buffer,
    pub read_bind_group: wgpu::BindGroup,
    pub read_bind_group_layout: wgpu::BindGroupLayout,
    pub compute_bind_group_layout: wgpu::BindGroupLayout,
    pub compute_bind_group: wgpu::BindGroup,
    pub compute_pipeline: wgpu::ComputePipeline,
}

impl PrecomputedDirections {
    pub fn new(device: &wgpu::Device, fov: f32) -> Self {
        let directions = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Directions Storage"),
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            mip_level_count: 1,
            sample_count: 1,
            size: wgpu::Extent3d {
                width: 1920*2,
                height: 1080*2,
                depth_or_array_layers: 1,
            },
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let ndc_multiplier = calc_ray_mult(fov, (1920*2, 1080*2));
        
        let ndc_mult = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Precompute Directions NDC Multiplier Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::bytes_of(&ndc_multiplier),
        });

        let view = directions.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Directions View"),
            format: Some(wgpu::TextureFormat::Rgba32Float),
            dimension: Some(wgpu::TextureViewDimension::D2),
            array_layer_count: None,
            aspect: wgpu::TextureAspect::All,
            base_array_layer: 0,
            base_mip_level: 0,
            mip_level_count: None,
            usage: None,
        });
        let read_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Precomputed Ray Directions Read Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::ReadOnly,
                        format: wgpu::TextureFormat::Rgba32Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                }
            ],
        });
        
        let read_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Precomputed Ray Directions Read Group"),
            layout: &read_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            }]
        });

        let compute_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Precomputed Ray Directions Compute Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba32Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        has_dynamic_offset: false,
                        min_binding_size: None,
                        ty: wgpu::BufferBindingType::Uniform,
                    },
                    count: None,
                }
            ],
        });

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Precomputed Ray Directions Write Group"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &ndc_mult,
                        offset: 0,
                        size: None,
                    }),
                }
            ]
        });

        let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Precompute Ray Directions Compute Pipeline Layout"),
            bind_group_layouts: &[&compute_bind_group_layout],
            push_constant_ranges: &[],
        });

        let precompute_shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/precompute_rays.wgsl"));

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Precompute Ray Directions Compute Pipeline"),
            cache: None,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            entry_point: Some("main"),
            layout: Some(&compute_pipeline_layout),
            module: &precompute_shader,
        });

        Self {
            directions,
            ndc_mult,
            read_bind_group,
            compute_bind_group,
            read_bind_group_layout,
            compute_bind_group_layout,
            compute_pipeline,
        }
    }

    pub fn compute(&self, compute_pass: &mut wgpu::ComputePass) {
        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
        compute_pass.dispatch_workgroups(240, 135, 1);
    }

    pub fn bind_read(&self, index: u32, compute_pass: &mut wgpu::ComputePass) {
        compute_pass.set_bind_group(index, &self.read_bind_group, &[]);
    }

    // pub fn bind_write(&self, index: u32, render_pass: &mut wgpu::RenderPass) {
    //     render_pass.set_bind_group(index, &self.compute_bind_group, &[]);
    // }

    // I don't think that I need this.
    // pub fn write_buffer(&self, directions: &[GpuVec3], queue: &wgpu::Queue) {
    //     queue.write_buffer(&self.directions, 0, bytemuck::cast_slice(directions));
    // }
}

pub struct GpuRaytraceResult {
    pub result_texture: wgpu::Texture,
    pub result_sampler: wgpu::Sampler,
    pub read_bind_group_layout: wgpu::BindGroupLayout,
    pub read_bind_group: wgpu::BindGroup,
    pub write_bind_group_layout: wgpu::BindGroupLayout,
    pub write_bind_group: wgpu::BindGroup,
    pub render_bind_group_layout: wgpu::BindGroupLayout,
    pub render_bind_group: wgpu::BindGroup,
    pub render_pipeline: wgpu::RenderPipeline,
}

impl GpuRaytraceResult {
    pub fn new(device: &wgpu::Device) -> Self {
        let result_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Raytrace Result Storage"),
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            mip_level_count: 1,
            sample_count: 1,
            size: wgpu::Extent3d {
                width: 1920*2,
                height: 1080*2,
                depth_or_array_layers: 1,
            },
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let result_storage_view = result_texture.create_view(&wgpu::TextureViewDescriptor {
            label: "Raytrace Result Storage Texture".into(),
            format: Some(wgpu::TextureFormat::Rgba8Unorm),
            dimension: Some(wgpu::TextureViewDimension::D2),
            array_layer_count: None,
            aspect: wgpu::TextureAspect::All,
            base_array_layer: 0,
            base_mip_level: 0,
            mip_level_count: None,
            usage: Some(wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING),
        });

        let result_render_view = result_texture.create_view(&wgpu::TextureViewDescriptor {
            label: "Raytrace Result Storage Render Texture".into(),
            format: Some(wgpu::TextureFormat::Rgba8Unorm),
            dimension: Some(wgpu::TextureViewDimension::D2),
            array_layer_count: None,
            aspect: wgpu::TextureAspect::All,
            base_array_layer: 0,
            base_mip_level: 0,
            mip_level_count: None,
            usage: Some(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING),
        });

        let result_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Raytrace Result Render Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let read_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Raytrace Result Read Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                ty: wgpu::BindingType::StorageTexture {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    access: wgpu::StorageTextureAccess::ReadOnly,
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                visibility: wgpu::ShaderStages::COMPUTE,
                count: None,
            }]
        });

        let read_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Raytrace Result Read Group"),
            layout: &read_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&result_storage_view),
            }]
        });
        let write_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Raytrace Result Write Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                ty: wgpu::BindingType::StorageTexture {
                    view_dimension: wgpu::TextureViewDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    access: wgpu::StorageTextureAccess::WriteOnly,
                },
                visibility: wgpu::ShaderStages::COMPUTE,
                count: None,
            }]
        });
        let write_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Raytrace Result Write Group"),
            layout: &write_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&result_storage_view),
            }]
        });
        let render_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Raytrace Result Render Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    count: None,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    visibility: wgpu::ShaderStages::FRAGMENT,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    count: None,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    visibility: wgpu::ShaderStages::FRAGMENT,
                },
            ]
        });

        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Raytrace Result Render Bind Group"),
            layout: &render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&result_render_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&result_sampler),
                },
            ]
        });

        let render_shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/raytrace_result_render.wgsl"));

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Raytrace Result Render Pipeline Layout"),
            bind_group_layouts: &[&render_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Raytrace Result Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &render_shader,
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                entry_point: Some("vertex_main"),
            },
            fragment: Some(wgpu::FragmentState {
                module: &render_shader,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                entry_point: Some("fragment_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                })],
            }),
            cache: None,
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                front_face: wgpu::FrontFace::Ccw,
                polygon_mode: wgpu::PolygonMode::Fill,
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
        });

        Self {
            result_texture,
            result_sampler,
            read_bind_group_layout,
            read_bind_group,
            write_bind_group_layout,
            write_bind_group,
            render_bind_group_layout,
            render_bind_group,
            render_pipeline,
        }
    }

    #[inline]
    pub fn bind_read(&self, index: u32, compute_pass: &mut wgpu::ComputePass) {
        compute_pass.set_bind_group(index, &self.read_bind_group, &[]);
    }

    #[inline]
    pub fn bind_write(&self, index: u32, compute_pass: &mut wgpu::ComputePass) {
        compute_pass.set_bind_group(index, &self.write_bind_group, &[]);
    }

    #[inline]
    pub fn bind_render(&self, index: u32, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_bind_group(index, &self.render_bind_group, &[]);
    }

    pub fn render(&self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_pipeline(&self.render_pipeline);
        self.bind_render(0, render_pass);
        render_pass.draw(0..6, 0..1);
    }
}

pub struct RaytraceChunk {
    blocks: Box<[u32]>,
    needs_write: bool,
}

impl RaytraceChunk {
    pub fn new() -> Self {
        Self {
            blocks: (0..64*64*64).map(|_| 0u32).collect(),
            needs_write: true,
        }
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> u32 {
        let xyz = x | y | z;
        if (xyz as u32) >= 64 {
            return 0;
        }

        let index = ((y << 12) | (z << 6) | x) as usize;
        self.blocks[index]
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, id: u32) {
        let xyz = x | y | z;
        if (xyz as u32) >= 64 {
            return;
        }

        let index = ((y << 12) | (z << 6) | x) as usize;
        self.blocks[index] = id;
        self.needs_write = true;
    }

    fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(self.blocks.as_ref())
    }
}

pub struct GpuRaytraceChunk {
    pub buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl GpuRaytraceChunk {
    pub fn new(chunk: &mut RaytraceChunk, device: &wgpu::Device) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Raytrace Chunk Buffer"),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            contents: chunk.as_bytes(),
        });
        chunk.needs_write = false;
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Raytrace Chunk Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                visibility: wgpu::ShaderStages::COMPUTE,
                count: None,
            }]
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Raytrace Chunkn Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }]
        });
        Self {
            buffer,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn bind(&self, index: u32, compute_pass: &mut wgpu::ComputePass) {
        compute_pass.set_bind_group(index, &self.bind_group, &[]);
    }

    pub fn write_chunk(&self, chunk: &RaytraceChunk, queue: &wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(chunk.blocks.as_ref()));
    }
}

pub struct Raytracer {
    // Result
    result: GpuRaytraceResult,
    // Chunk
    pub chunk: RaytraceChunk,
    gpu_chunk: GpuRaytraceChunk,
    // Camera
    gpu_camera: RaytraceCamera,
    // Directions
    gpu_precompute: PrecomputedDirections,
    // Pipelines
    raytrace_pipeline: wgpu::ComputePipeline,
}

impl Raytracer {
    pub fn new(camera: &Camera, chunk: Option<RaytraceChunk>, device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let result = GpuRaytraceResult::new(device);
        let mut chunk = chunk.unwrap_or_else(|| RaytraceChunk::new());
        let gpu_chunk = GpuRaytraceChunk::new(&mut chunk, device);
        gpu_chunk.write_chunk(&chunk, queue);
        let gpu_camera = RaytraceCamera::new(camera, device);
        let gpu_precompute = PrecomputedDirections::new(device, camera.fov);

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Precompute Compute Pass"),
            timestamp_writes: None,
        });

        gpu_precompute.compute(&mut compute_pass);
        drop(compute_pass);
        let command_buffer = encoder.finish();
        queue.submit(Some(command_buffer));

        let raytrace_shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/raytrace.wgsl"));

        let raytrace_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Raytracer Compute Pipeline Layout"),
            bind_group_layouts: &[
                &result.write_bind_group_layout,
                &gpu_chunk.bind_group_layout,
                &gpu_camera.bind_group_layout,
                &gpu_precompute.read_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });
        let raytrace_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Raytracer Compute Pipeline"),
            module: &raytrace_shader,
            cache: None,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            entry_point: Some("main"),
            layout: Some(&raytrace_pipeline_layout),
        });
        Self {
            result,
            chunk,
            gpu_chunk,
            gpu_camera,
            gpu_precompute,
            raytrace_pipeline,
        }
    }

    pub fn write_chunk(&mut self, queue: &wgpu::Queue) {
        self.gpu_chunk.write_chunk(&self.chunk, queue);
        self.chunk.needs_write = false;
    }

    pub fn write_camera_transform(&mut self, transform: GpuTransform, queue: &wgpu::Queue) {
        self.gpu_camera.write_transform(transform, queue);
    }

    pub fn compute(&self, compute_pass: &mut wgpu::ComputePass) {
        compute_pass.set_pipeline(&self.raytrace_pipeline);
        self.result.bind_write(0, compute_pass);
        self.gpu_chunk.bind(1, compute_pass);
        self.gpu_camera.bind(2, compute_pass);
        self.gpu_precompute.bind_read(3, compute_pass);
        compute_pass.dispatch_workgroups(240, 135, 1);
    }

    pub fn render(&self, render_pass: &mut wgpu::RenderPass) {
        self.result.render(render_pass);
    }

}