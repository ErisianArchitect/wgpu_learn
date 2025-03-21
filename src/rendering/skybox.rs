use std::path::Path;

use std::sync::Arc;

use glam::{vec2, vec3, Vec3};
use image::GenericImageView;
use wgpu::util::DeviceExt;

use crate::{modeling::modeler::{Modeler, PosUV}, voxel::vertex::Vertex};

use super::transforms::TransformsBindGroup;

#[derive(Debug, thiserror::Error)]
pub enum SkyboxErr {
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to load image: {0}")]
    FailedToLoadImage(#[from] image::ImageError),
    #[error("{side} has dimensions of {dimensions:?}, expected {expected:?}.")]
    MismatchedDimensions {
        side: &'static str,
        dimensions: (u32, u32),
        expected: (u32, u32),
    }
}

#[derive(Debug, Clone)]
struct SkyboxInner {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    render_pipeline: wgpu::RenderPipeline,
    num_indices: u32,
    cubemap: SkyboxCubemap,
}

#[derive(Debug, Clone)]
pub struct Skybox {
    inner: Arc<SkyboxInner>,
}

pub struct SkyboxTexturePaths<P: AsRef<Path>> {
    pub top: P,
    pub bottom: P,
    pub front: P,
    pub back: P,
    pub left: P,
    pub right: P,
}

#[derive(Debug, Clone)]
pub struct SkyboxCubemap {
    pub cubemap: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub format: wgpu::TextureFormat,
    pub dimensions: (u32, u32),
    pub binding: SkyboxCubemapBinding,
}

impl SkyboxCubemap {

    pub fn load<P: AsRef<std::path::Path>>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: Option<&str>,
        format: wgpu::TextureFormat,
        paths: &SkyboxTexturePaths<P>,
    ) -> Result<Self, SkyboxErr> {
        let paths = [
            paths.right.as_ref(),
            paths.left.as_ref(),
            paths.top.as_ref(),
            paths.bottom.as_ref(),
            paths.front.as_ref(),
            paths.back.as_ref(),
        ];
        let reader = image::ImageReader::open(paths[0])?;
        let (width, height) = reader.into_dimensions()?;

        let cubemap = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Skybox Cubemap Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        const SIDE_NAMES: [&'static str; 6] = [
            "Right",
            "Left",
            "Top",
            "Bottom",
            "Front",
            "Back",
        ];

        let bytes_per_row = Some(4 * width);
        let rows_per_image = Some(height);

        for (i, img_path) in paths.into_iter().enumerate() {
            let img = image::open(img_path)?;
            let (img_width, img_height) = img.dimensions();
            if (img_width, img_height) != (width, height) {
                return Err(SkyboxErr::MismatchedDimensions {
                    side: SIDE_NAMES[i],
                    dimensions: (img_width, img_height),
                    expected: (width, height)
                });
            }
            let img_rgba = img.to_rgba8();

            queue.write_texture(
                wgpu::TexelCopyTextureInfoBase {
                    texture: &cubemap,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: i as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &img_rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row,
                    rows_per_image,
                },
                wgpu::Extent3d {
                    width,
                    height, depth_or_array_layers: 1,
                },
            );
        }

        let view = cubemap.create_view(&wgpu::TextureViewDescriptor {
            label,
            format: Some(format),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
            ..Default::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let binding = SkyboxCubemapBinding::new(device, &view, &sampler);

        Ok(Self {
            cubemap,
            view,
            sampler,
            format,
            dimensions: (width, height),
            binding,
        })
    }

    pub fn bind(&self, index: u32, render_pass: &mut wgpu::RenderPass) {
        self.binding.bind(index, render_pass);
    }
}

#[derive(Debug, Clone)]
pub struct SkyboxCubemapBinding {
    pub layout: wgpu::BindGroupLayout,
    pub group: wgpu::BindGroup,
}

impl SkyboxCubemapBinding {
    pub fn new(
        device: &wgpu::Device,
        view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Skybox Cubemap Texture Bind Group Layout"),
            entries: &[
                // cubemap
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                // sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Skybox Cubemap Texture Bind Group"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });

        Self {
            layout,
            group,
        }
    }

    pub fn bind(&self, index: u32, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_bind_group(index, &self.group, &[]);
    }
}

impl Skybox {
    pub const TOP_INDEX: u32 = 0;
    pub const BOTTOM_INDEX: u32 = 1;
    pub const LEFT_INDEX: u32 = 2;
    pub const RIGHT_INDEX: u32 = 3;
    pub const FRONT_INDEX: u32 = 4;
    pub const BACK_INDEX: u32 = 5;
    pub fn new<P: AsRef<Path>>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
        label: Option<&str>,
        format: wgpu::TextureFormat,
        transforms: &TransformsBindGroup,
        paths: &SkyboxTexturePaths<P>,
    ) -> Result<Self, SkyboxErr> {
        let cubemap = SkyboxCubemap::load(device, queue, label, format, paths)?;
        // top, bottom, left, right, front, back
        let mut m = Modeler::new();
        let quad = [
            PosUV::new(vec3(0.5, -0.5, -0.5), vec2(0.0, 0.0)),PosUV::new(vec3(-0.5, -0.5, -0.5), vec2(1.0, 0.0)),
            PosUV::new(vec3(0.5, 0.5, -0.5), vec2(0.0, 1.0)),PosUV::new(vec3(-0.5, 0.5, -0.5), vec2(1.0, 1.0)),
        ];
        m.texture_index(Self::FRONT_INDEX, |m| {
            m.push_quad(&quad);
            m.rotate_euler(glam::EulerRot::XYZ, vec3(270.0f32.to_radians(), 0.0, 0.0), |m| {
                m.push_quad(&quad);
            });
            m.rotate_euler(glam::EulerRot::XYZ, vec3(90.0f32.to_radians(), 0.0, 0.0), |m| {
                m.push_quad(&quad);
            });
            m.rotate_euler(glam::EulerRot::XYZ, vec3(0.0, 90.0f32.to_radians(), 0.0), |m| {
                m.push_quad(&quad);
            });
            m.rotate_euler(glam::EulerRot::XYZ, vec3(0.0, 180.0f32.to_radians(), 0.0), |m| {
                m.push_quad(&quad);
            });
            m.rotate_euler(glam::EulerRot::XYZ, vec3(0.0, 270.0f32.to_radians(), 0.0), |m| {
                m.push_quad(&quad);
            });
        });
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Skybox Vertex Buffer"),
            contents: bytemuck::cast_slice(m.vertices.as_slice()),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Skybox Index Buffer"),
            contents: bytemuck::cast_slice(m.indices.as_slice()),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = m.indices.len() as u32;
        let shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/skybox.wgsl"));
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Skybox Render Pipeline Layout"),
            bind_group_layouts: &[
                &transforms.bind_group_layout,
                &cubemap.binding.layout,
            ],
            push_constant_ranges: &[wgpu::PushConstantRange {
                range: 0..64,
                stages: wgpu::ShaderStages::VERTEX,
            }],
        });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Skybox Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    Vertex::desc(),
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            // depth_stencil: Some(wgpu::DepthStencilState {
            //     format: wgpu::TextureFormat::Depth32Float,
            //     depth_write_enabled: true,
            //     depth_compare: wgpu::CompareFunction::Less,
            //     stencil: wgpu::StencilState::default(),
            //     bias: wgpu::DepthBiasState::default(),
            // }),
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Ok(Self {
            inner: Arc::new(SkyboxInner {
                vertex_buffer,
                index_buffer,
                render_pipeline,
                num_indices,
                cubemap,
            })
        })
    }

    pub fn render(
        &self,
        render_pass: &mut wgpu::RenderPass,
        transforms: &TransformsBindGroup,
        camera_position: Vec3,
    ) {
        render_pass.set_pipeline(&self.inner.render_pipeline);
        render_pass.set_bind_group(0, &transforms.bind_group, &[]);
        self.inner.cubemap.bind(1, render_pass);

        render_pass.set_vertex_buffer(0, self.inner.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.inner.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        let world = glam::Mat4::from_translation(camera_position);
        render_pass.set_push_constants(wgpu::ShaderStages::VERTEX, 0, bytemuck::bytes_of(&world));
        render_pass.draw_indexed(0..self.inner.num_indices, 0, 0..1);
    }
}