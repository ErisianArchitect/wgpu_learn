use std::{cell::RefCell, fs::File, io::BufWriter, path::Path};

use glam::*;
use bytemuck::{NoUninit, Pod, Zeroable};
use wgpu::util::DeviceExt;
use crate::{camera::Camera, math::{ray::Ray3, *}};

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

// #[repr(C)]
// #[derive(Debug, Clone, Copy, NoUninit)]
// pub struct RayHit {
//     pub coord: IVec3,
//     _coord_pad: [u8; 4],
//     pub distance: f32,
//     pub id: u32,
//     pub face: Face,
//     pub hit: bool,
//     _hit_pad: [u8; 3],
// }

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
    // pub bind_group: wgpu::BindGroup,
    // pub bind_group_layout: wgpu::BindGroupLayout,
}

impl RaytraceCamera {
    pub fn new(camera: &Camera, device: &wgpu::Device) -> Self {
        let gpu_cam = GpuRaytraceCamera::new(camera, 0.1, 1000.0);
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Raytrace Camera Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::bytes_of(&gpu_cam),
        });
        // let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        //     label: Some("Raytrace Camera Layout"),
        //     entries: &[wgpu::BindGroupLayoutEntry {
        //         binding: 0,
        //         ty: wgpu::BindingType::Buffer {
        //             ty: wgpu::BufferBindingType::Uniform,
        //             has_dynamic_offset: false,
        //             min_binding_size: None,
        //         },
        //         visibility: wgpu::ShaderStages::COMPUTE,
        //         count: None,
        //     }],
        // });
        // let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     label: Some("Raytrace Camera Group"),
        //     layout: &bind_group_layout,
            // entries: &[wgpu::BindGroupEntry {
            //     binding: 0,
            //     resource: buffer.as_entire_binding(),
            // }]
        // });
        Self {
            gpu_cam,
            buffer,
            // bind_group,
            // bind_group_layout,
        }
    }

    // pub fn bind(&self, index: u32, compute_pass: &mut wgpu::ComputePass) {
    //     compute_pass.set_bind_group(index, &self.bind_group, &[]);
    // }

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
                width: 1920,
                height: 1080,
                depth_or_array_layers: 1,
            },
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let ndc_multiplier = calc_ray_mult(fov, (1920, 1080));
        
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
        compute_pass.dispatch_workgroups(120, 68, 1);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Axis {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Face {
    PosX = 0,
    PosY = 1,
    PosZ = 2,
    NegX = 3,
    NegY = 4,
    NegZ = 5,
}

impl Face {
    #[inline]
    pub fn axis(self) -> Axis {
        match self {
            Face::PosX | Face::NegX => Axis::X,
            Face::PosY | Face::NegY => Axis::Y,
            Face::PosZ | Face::NegZ => Axis::Z,
        }
    }

    #[inline]
    pub const fn normal(self) -> Vec3A {
        match self {
            Face::PosX => Vec3A::X,
            Face::PosY => Vec3A::Y,
            Face::PosZ => Vec3A::Z,
            Face::NegX => Vec3A::NEG_X,
            Face::NegY => Vec3A::NEG_Y,
            Face::NegZ => Vec3A::NEG_Z,
        }
    }

    pub fn from_direction(dir: Vec3A) -> Self {
        let abs = dir.abs();
        if abs.x >= abs.y {
            if abs.x >= abs.z {
                if dir.x.is_sign_negative() {
                    Face::NegX
                } else {
                    Face::PosX
                }
            } else {
                if dir.z.is_sign_negative() {
                    Face::NegZ
                } else {
                    Face::PosZ
                }
            }
        } else {
            if abs.y >= abs.z {
                if dir.y.is_sign_negative() {
                    Face::NegY
                } else {
                    Face::PosY
                }
            } else {
                if dir.z.is_sign_negative() {
                    Face::NegZ
                } else {
                    Face::PosZ
                }
            }
        }
    }

    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone)]
pub struct RayHit {
    pub coord: IVec3,
    pub id: u32,
    pub face: Option<Face>,
    pub distance: f32,
}

impl RayHit {
    #[inline(always)]
    pub fn hit_face(coord: IVec3, distance: f32, id: u32, face: Face) -> Self {
        Self {
            coord,
            distance,
            id,
            face: Some(face),
        }
    }

    #[inline(always)]
    pub fn hit_cell(coord: IVec3, id: u32, distance: f32) -> Self {
        Self {
            coord,
            distance,
            id,
            face: None,
        }
    }

    #[inline(always)]
    pub fn get_hit_point(&self, ray: Ray3, face: Face) -> Vec3A {
        let point = ray.point_on_ray(self.distance);
        let pre_hit = match face {
            Face::PosX => ivec3(self.coord.x + 1, self.coord.y, self.coord.z),
            Face::PosY => ivec3(self.coord.x, self.coord.y + 1, self.coord.z),
            Face::PosZ => ivec3(self.coord.x, self.coord.y, self.coord.z + 1),
            Face::NegX => ivec3(self.coord.x - 1, self.coord.y, self.coord.z),
            Face::NegY => ivec3(self.coord.x, self.coord.y - 1, self.coord.z),
            Face::NegZ => ivec3(self.coord.x, self.coord.y, self.coord.z - 1),
        };
        let pre_hit = pre_hit.as_vec3a();
        const SMIDGEN: Vec3A = Vec3A::splat(1e-3);
        const UNSMIDGEN: Vec3A = Vec3A::splat(1.0-1e-3);
        // sometimes the hit-point is in the wrong cell (if it goes too far)
        // so you want to bring it back into the correct cell.
        let min = pre_hit + SMIDGEN;
        let max = pre_hit + UNSMIDGEN;
        // point.max(min).min(max)
        point.clamp(min, max)
    }

    pub fn get_hit_cell(&self) -> IVec3 {
        let mut coord = self.coord;
        match self.face {
            Some(Face::NegX) => {
                coord.x -= 1;
            }
            Some(Face::NegY) => {
                coord.y -= 1;
            }
            Some(Face::NegZ) => {
                coord.z -= 1;
            }
            Some(Face::PosX) => {
                coord.x += 1;
            }
            Some(Face::PosY) => {
                coord.y += 1;
            }
            Some(Face::PosZ) => {
                coord.z += 1;
            }
            None => ()
        }
        coord
    }
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
                width: 1920,
                height: 1080,
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
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
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
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
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

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), std::io::Error> {
        use std::{fs::File, io::{ Write, BufWriter }};
        let path = path.as_ref();
        std::fs::create_dir_all(path.parent().unwrap())?;
        let file = File::create(path)?;
        let mut buffer = BufWriter::new(file);
        for i in 0..self.blocks.len() {
            buffer.write_all(&self.blocks[i].to_be_bytes())?;
        }
        Ok(())
    }

    pub fn load<P: AsRef<Path>>(&mut self, path: P) -> Result<(), std::io::Error> {
        use std::{fs::File, io::{ Read, BufReader}};
        let path = path.as_ref();
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        for i in 0..self.blocks.len() {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf)?;
            self.blocks[i] = u32::from_be_bytes(buf);
        }
        self.needs_write = true;
        Ok(())
    }

    pub fn raycast(&self, ray: Ray3, max_distance: f32) -> Option<RayHit> {
        let mut ray = ray;
        let lt = ray.pos.cmplt(Vec3A::ZERO);
        const SIXTY_FOUR: Vec3A = Vec3A::splat(64.0);
        let ge = ray.pos.cmpge(SIXTY_FOUR);
        let outside = lt | ge;
        let (step, delta_min, delta_max, delta_add) = if outside.any() {
            // Calculate entry point (if there is one).
            // calculate distance to cross each plane
            let sign = ray.dir.signum();
            let step = sign.as_ivec3();

            let neg_sign = sign.cmplt(Vec3A::ZERO);
            let pos_sign = sign.cmpgt(Vec3A::ZERO);

            if ((lt & neg_sign) | (ge & pos_sign)).any() {
                return None;
            }
            // if lt.test(0) && step.x < 0 // 4
            // || lt.test(1) && step.y < 0 // 5 9
            // || lt.test(2) && step.z < 0 // 5 14
            // || ge.test(0) && step.x > 0 // 5 19
            // || ge.test(1) && step.y > 0 // 5 24
            // || ge.test(2) && step.z > 0 {// 5 29
            //     return None;
            // }
            let (dx_min, dx_max) = match step.x + 1 {
                0 => {
                    (
                        (ray.pos.x - 64.0) / -ray.dir.x,
                        ray.pos.x / -ray.dir.x,
                    )
                }
                1 => {
                    (<f32>::NEG_INFINITY, <f32>::INFINITY)
                }
                2 => {
                    (
                        -ray.pos.x / ray.dir.x,
                        (64.0 - ray.pos.x) / ray.dir.x,
                    )
                }
                _ => unreachable!(),
            };
            let (dy_min, dy_max) = match step.y + 1 {
                0 => {
                    (
                        (ray.pos.y - 64.0) / -ray.dir.y,
                        ray.pos.y / -ray.dir.y,
                    )
                }
                1 => {
                    (<f32>::NEG_INFINITY, <f32>::INFINITY)
                }
                2 => {
                    (
                        -ray.pos.y / ray.dir.y,
                        (64.0 - ray.pos.y) / ray.dir.y,
                    )
                }
                _ => unreachable!()
            };
            let (dz_min, dz_max) = match step.z + 1 {
                0 => {
                    (
                        (ray.pos.z - 64.0) / -ray.dir.z,
                        ray.pos.z / -ray.dir.z,
                    )
                }
                1 => {
                    (<f32>::NEG_INFINITY, <f32>::INFINITY)
                }
                2 => {
                    (
                        -ray.pos.z / ray.dir.z,
                        (64.0 - ray.pos.z) / ray.dir.z,
                    )
                }
                _ => unreachable!()
            };
            let max_min = dx_min.max(dy_min.max(dz_min));
            let min_max = dx_max.min(dy_max.min(dz_max));
            // Early return, the ray does not hit the volume.
            if max_min >= min_max {
                return None;
            }
            // This is needed to penetrate the ray into the bounding box.
            // Otherwise you'll get weird circles from the rays popping
            // in and out of the next cell. This ensures that the ray
            // will be inside the bounding box.
            const RAY_PENETRATION: f32 = 1e-5;
            let delta_add = max_min + RAY_PENETRATION;
            if delta_add >= max_distance {
                return None;
            }
            ray.pos = ray.pos + ray.dir * delta_add;
            (
                step,
                Some(vec3(dx_min, dy_min, dz_min)),
                vec3(dx_max, dy_max, dz_max),
                delta_add,
            )
        } else {
            let sign = ray.dir.signum();
            let step = sign.as_ivec3();
            let dx_max = match step.x + 1 {
                0 => {
                    ray.pos.x / -ray.dir.x
                }
                1 => {
                    <f32>::INFINITY
                }
                2 => {
                    (64.0 - ray.pos.x) / ray.dir.x
                }
                _ => unreachable!()
            };
            let dy_max = match step.y + 1 {
                0 => {
                    ray.pos.y / -ray.dir.y
                }
                1 => {
                    <f32>::INFINITY
                }
                2 => {
                    (64.0 - ray.pos.y) / ray.dir.y
                }
                _ => unreachable!()
            };
            let dz_max = match step.z + 1{
                0 => {
                    ray.pos.z / -ray.dir.z
                }
                1 => {
                    <f32>::INFINITY
                }
                2 => {
                    (64.0 - ray.pos.z) / ray.dir.z
                }
                _ => unreachable!()
            };
            (
                step,
                None,
                vec3(dx_max, dy_max, dz_max),
                0.0,
            )
        };
        #[inline(always)]
        fn calc_delta(mag: f32) -> f32 {
            1.0 / mag.abs().max(<f32>::MIN_POSITIVE)
        }
        let delta = vec3(
            calc_delta(ray.dir.x),
            calc_delta(ray.dir.y),
            calc_delta(ray.dir.z),
        );

        let face = (
            if step.x >= 0 {
                Face::NegX
            } else {
                Face::PosX
            },
            if step.y >= 0 {
                Face::NegY
            } else {
                Face::PosY
            },
            if step.z >= 0 {
                Face::NegZ
            } else {
                Face::PosZ
            },
        );

        let fract = ray.pos.fract();

        #[inline(always)]
        fn calc_t_max(step: i32, fract: f32, mag: f32) -> f32 {
            if step > 0 {
                (1.0 - fract) / mag.abs().max(<f32>::MIN_POSITIVE)
            } else if step < 0 {
                fract / mag.abs().max(<f32>::MIN_POSITIVE)
            } else {
                <f32>::INFINITY
            }
        }
        let mut t_max = vec3(
            calc_t_max(step.x, fract.x, ray.dir.x) + delta_add,
            calc_t_max(step.y, fract.y, ray.dir.y) + delta_add,
            calc_t_max(step.z, fract.z, ray.dir.z) + delta_add,
        );

        let mut cell = ray.pos.floor().as_ivec3();
        let id = self.get(cell.x, cell.y, cell.z);
        if id != 0 {
            return Some(RayHit {
                face: delta_min.map(|min| {
                    if min.x >= min.y {
                        if min.x >= min.z {
                            face.0
                        } else {
                            face.2
                        }
                    } else {
                        if min.y >= min.z {
                            face.1
                        } else {
                            face.2
                        }
                    }
                }),
                id,
                coord: cell,
                distance: delta_add,
            });
        }
        let max_d = vec3a(
            delta_max.x.min(max_distance),
            delta_max.y.min(max_distance),
            delta_max.z.min(max_distance),
        );
        loop {

            if t_max.x <= t_max.y {
                if t_max.x <= t_max.z {
                    if t_max.x >= max_d.x {
                        return None;
                    }
                    cell.x += step.x;
                    let id = self.get(cell.x, cell.y, cell.z);
                    if id != 0 {
                        return Some(RayHit::hit_face(cell, t_max.x, id, face.0));
                    }
                    t_max.x += delta.x;
                } else {
                    if t_max.z >= max_d.z {
                        return None;
                    }
                    cell.z += step.z;
                    let id = self.get(cell.x, cell.y, cell.z);
                    if id != 0 {
                        return Some(RayHit::hit_face(cell, t_max.z, id, face.2));
                    }
                    t_max.z += delta.z;
                }
            } else {
                if t_max.y <= t_max.z {
                    if t_max.y >= max_d.y {
                        return None;
                    }
                    cell.y += step.y;
                    let id = self.get(cell.x, cell.y, cell.z);
                    if id != 0 {
                        return Some(RayHit::hit_face(cell, t_max.y, id, face.1));
                    }
                    t_max.y += delta.y;
                } else {
                    if t_max.z >= max_d.z {
                        return None;
                    }
                    cell.z += step.z;
                    let id = self.get(cell.x, cell.y, cell.z);
                    if id != 0 {
                        return Some(RayHit::hit_face(cell, t_max.z, id, face.2));
                    }
                    t_max.z += delta.z;
                }
            }
        }
    }
}

pub struct GpuRaytraceChunk {
    pub buffer: wgpu::Buffer,
    // pub bind_group_layout: wgpu::BindGroupLayout,
    // pub bind_group: wgpu::BindGroup,
}

impl GpuRaytraceChunk {
    pub fn new(chunk: &mut RaytraceChunk, device: &wgpu::Device) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Raytrace Chunk Buffer"),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            contents: chunk.as_bytes(),
        });
        chunk.needs_write = false;
        // let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        //     label: Some("Raytrace Chunk Layout"),
            // entries: &[wgpu::BindGroupLayoutEntry {
            //     binding: 0,
            //     ty: wgpu::BindingType::Buffer {
            //         ty: wgpu::BufferBindingType::Storage { read_only: true },
            //         has_dynamic_offset: false,
            //         min_binding_size: None,
            //     },
            //     visibility: wgpu::ShaderStages::COMPUTE,
            //     count: None,
            // }]
        // });
        // let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     label: Some("Raytrace Chunk Group"),
        //     layout: &bind_group_layout,
            // entries: &[wgpu::BindGroupEntry {
            //     binding: 0,
            //     resource: buffer.as_entire_binding(),
            // }]
        // });
        Self {
            buffer,
            // bind_group_layout,
            // bind_group,
        }
    }

    // pub fn bind(&self, index: u32, compute_pass: &mut wgpu::ComputePass) {
    //     compute_pass.set_bind_group(index, &self.bind_group, &[]);
    // }

    pub fn write_chunk(&self, chunk: &RaytraceChunk, queue: &wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(chunk.blocks.as_ref()));
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, NoUninit)]
pub struct RtDirectionalLight {
    direction: Vec3,
    _pad0: [u8; 4],
    color: Vec3,
    evening_intensity: f32,
    intensity: f32,
    shadow: f32,
    active: bool,
    _pad2: [u8; 7],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, NoUninit)]
pub struct RtAmbientLight {
    color: Vec3,
    _pad0: [u8; 4],
    intensity: f32,
    active: bool,
    _pad1: [u8; 11],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, NoUninit)]
pub struct RtLighting {
    directional: RtDirectionalLight,
    ambient: RtAmbientLight,
}

pub struct GpuRtLighting {
    lighting: RefCell<RtLighting>,
    buffer: wgpu::Buffer,
    // bind_group_layout: wgpu::BindGroupLayout,
    // bind_group: wgpu::BindGroup,
}

pub struct DirectionalLight {
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub evening_intensity: f32,
    pub shadow: f32,
    pub active: bool,
}

pub struct AmbientLight {
    pub color: Vec3,
    pub intensity: f32,
    pub active: bool,
}

pub struct Lighting {
    pub directional: DirectionalLight,
    pub ambient: AmbientLight,
}

impl GpuRtLighting {
    pub fn new(device: &wgpu::Device, lighting: &Lighting) -> Self {
        let rt_light = RtLighting {
            directional: RtDirectionalLight {
                direction: lighting.directional.direction,
                color: lighting.directional.color,
                intensity: lighting.directional.intensity,
                evening_intensity: lighting.directional.evening_intensity,
                shadow: lighting.directional.shadow,
                active: lighting.directional.active,
                _pad0: padding(),
                _pad2: padding(),
            },
            ambient: RtAmbientLight {
                color: lighting.ambient.color,
                intensity: lighting.ambient.intensity,
                active: lighting.ambient.active,
                _pad0: padding(),
                _pad1: padding(),
            },
        };
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GPU Lighting Buffer"),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::bytes_of(&rt_light),
        });

        // let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        //     label: Some("GPU Lighting Bind Group Layout"),
        //     entries: &[
        //         wgpu::BindGroupLayoutEntry {
        //             binding: 0,
        //             count: None,
        //             visibility: wgpu::ShaderStages::COMPUTE,
        //             ty: wgpu::BindingType::Buffer {
        //                 min_binding_size: None,
        //                 has_dynamic_offset: false,
        //                 ty: wgpu::BufferBindingType::Uniform,
        //             }
        //         }
        //     ]
        // });

        // let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     label: Some("GPU Lighting Bind Group"),
        //     layout: &bind_group_layout,
        //     entries: &[
        //         wgpu::BindGroupEntry {
        //             binding: 0,
        //             resource: buffer.as_entire_binding(),
        //         }
        //     ]
        // });

        Self {
            lighting: RefCell::new(rt_light),
            buffer,
            // bind_group_layout,
            // bind_group,
        }

    }

    pub fn set_directional_direction(&self, queue: &wgpu::Queue, direction: Vec3) {
        let mut lighting = self.lighting.borrow_mut();
        lighting.directional.direction = direction;
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&direction));
    }

    pub fn get_directional_direction(&self) -> Vec3 {
        self.lighting.borrow().directional.direction
    }

    pub fn set_directional_color(&self, queue: &wgpu::Queue, color: Vec3) {
        let mut lighting = self.lighting.borrow_mut();
        lighting.directional.color = color;
        queue.write_buffer(&self.buffer, 16, bytemuck::bytes_of(&color));
    }

    pub fn get_directional_color(&self) -> Vec3 {
        self.lighting.borrow().directional.color
    }

    pub fn set_directional_intensity(&self, queue: &wgpu::Queue, intensity: f32) {
        let mut lighting = self.lighting.borrow_mut();
        lighting.directional.intensity = intensity;
        queue.write_buffer(&self.buffer, 32, bytemuck::bytes_of(&intensity));
    }

    pub fn get_directional_intensity(&self) -> f32 {
        self.lighting.borrow().directional.intensity
    }

    pub fn set_shadow(&self, queue: &wgpu::Queue, shadow: f32) {
        let mut lighting = self.lighting.borrow_mut();
        lighting.directional.shadow = shadow;
        queue.write_buffer(&self.buffer, 36, bytemuck::bytes_of(&shadow));
    }

    pub fn get_shadow(&self) -> f32 {
        self.lighting.borrow().directional.shadow
    }

    pub fn set_directional_active(&self, queue: &wgpu::Queue, active: bool) {
        let mut lighting = self.lighting.borrow_mut();
        lighting.directional.active = active;
        queue.write_buffer(&self.buffer, 40, bytemuck::bytes_of(&active));
    }

    pub fn get_directional_active(&self) -> bool {
        self.lighting.borrow().directional.active
    }

    pub fn set_ambient_color(&self, queue: &wgpu::Queue, color: Vec3) {
        let mut lighting = self.lighting.borrow_mut();
        lighting.ambient.color = color;
        queue.write_buffer(&self.buffer, 48, bytemuck::bytes_of(&color));
    }

    pub fn get_ambient_color(&self) -> Vec3 {
        self.lighting.borrow().ambient.color
    }

    pub fn set_ambient_intensity(&self, queue: &wgpu::Queue, intensity: f32) {
        let mut lighting = self.lighting.borrow_mut();
        lighting.ambient.intensity = intensity;
        queue.write_buffer(&self.buffer, 64, bytemuck::bytes_of(&intensity));
    }

    pub fn get_ambient_intensity(&self) -> f32 {
        self.lighting.borrow().ambient.intensity
    }

    pub fn set_ambient_active(&self, queue: &wgpu::Queue, active: bool) {
        let mut lighting = self.lighting.borrow_mut();
        lighting.ambient.active = active;
        queue.write_buffer(&self.buffer, 68, bytemuck::bytes_of(&active));
    }

    pub fn get_abmient_active(&self) -> bool {
        self.lighting.borrow().ambient.active
    }

    // fn bind(&self, index: u32, compute_pass: &mut wgpu::ComputePass) {
    //     compute_pass.set_bind_group(index, &self.bind_group, &[]);
    // }
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
    // Lighting
    pub gpu_lighting: GpuRtLighting,
    data_bind_group_layout: wgpu::BindGroupLayout,
    data_bind_group: wgpu::BindGroup,
    // Pipelines
    raytrace_pipeline: wgpu::ComputePipeline,
}

impl Raytracer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, camera: &Camera, chunk: Option<RaytraceChunk>, lighting: &Lighting) -> Self {
        let result = GpuRaytraceResult::new(device);
        let mut chunk = chunk.unwrap_or_else(|| RaytraceChunk::new());
        let gpu_chunk = GpuRaytraceChunk::new(&mut chunk, device);
        gpu_chunk.write_chunk(&chunk, queue);
        let mut gpu_camera = RaytraceCamera::new(camera, device);
        gpu_camera.write_dimensions(1920, 1080, queue);
        let gpu_precompute = PrecomputedDirections::new(device, camera.fov);
        let gpu_lighting = GpuRtLighting::new(device, lighting);

        let data_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Raytracer Data Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    visibility: wgpu::ShaderStages::COMPUTE,
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    visibility: wgpu::ShaderStages::COMPUTE,
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    count: None,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        min_binding_size: None,
                        has_dynamic_offset: false,
                        ty: wgpu::BufferBindingType::Uniform,
                    }
                },
            ]
        });

        let data_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Raytracer Data Bind Group"),
            layout: &data_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: gpu_camera.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: gpu_chunk.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: gpu_lighting.buffer.as_entire_binding(),
                },
            ]
        });

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
                &gpu_precompute.read_bind_group_layout,
                &data_bind_group_layout,
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
            gpu_lighting,
            data_bind_group_layout,
            data_bind_group,
            raytrace_pipeline,
        }
    }

    pub fn write_chunk(&mut self, queue: &wgpu::Queue) {
        if !self.chunk.needs_write {
            return;
        }
        self.gpu_chunk.write_chunk(&self.chunk, queue);
        self.chunk.needs_write = false;
    }

    pub fn write_camera_transform(&mut self, transform: GpuTransform, queue: &wgpu::Queue) {
        self.gpu_camera.write_transform(transform, queue);
    }

    pub fn compute(&self, compute_pass: &mut wgpu::ComputePass, query_set: Option<&wgpu::QuerySet>) {
        compute_pass.set_pipeline(&self.raytrace_pipeline);
        self.result.bind_write(0, compute_pass);
        self.gpu_precompute.bind_read(1, compute_pass);
        compute_pass.set_bind_group(2, &self.data_bind_group, &[]);
        // self.gpu_chunk.bind(2, compute_pass);
        // self.gpu_camera.bind(3, compute_pass);
        // self.gpu_lighting.bind(4, compute_pass);
        match query_set {
            Some(query_set) => {
                compute_pass.write_timestamp(query_set, 0);
                compute_pass.dispatch_workgroups(240, 135, 1);
                compute_pass.write_timestamp(query_set, 1);
            },
            None => {
                compute_pass.dispatch_workgroups(240, 135, 1);
            },
        }
        
    }

    pub fn render(&self, render_pass: &mut wgpu::RenderPass) {
        self.result.render(render_pass);
    }

}