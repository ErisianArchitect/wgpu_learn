use vello::{Renderer, RendererOptions};

use crate::state::State;

pub struct Velvet {
    pub renderer: Renderer,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    // sampler: wgpu::Sampler,
    // bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub draw_pipeline: wgpu::RenderPipeline,
}

impl Velvet {
    pub fn new(device: &wgpu::Device) -> Self {
        let renderer = Renderer::new(
            device,
            RendererOptions::default(),
        ).expect("Failed to create renderer.");
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Velvet Render Texture"),
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            mip_level_count: 1,
            sample_count: 1,
            size: wgpu::Extent3d {
                depth_or_array_layers: 1,
                width: 1280,
                height: 720,
            },
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Velvet Render Texture Sampler"),
            mipmap_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Velvet Render Bind Group Layout"),
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
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Velvet Render Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                }
            ],
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("../shaders/stretch_texture.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Velvet Render Pipeline Layout"),
            bind_group_layouts: &[
                &bind_group_layout,
            ],
            ..Default::default()
        });

        let draw_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Velvet Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                entry_point: Some("vertex_main"),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                entry_point: Some("fragment_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::all(),
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                })]
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
            renderer,
            // sampler,
            texture,
            view,
            // bind_group_layout,
            bind_group,
            draw_pipeline,
        }
    }

    pub fn draw<F: FnMut(&mut vello::Scene)>(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, mut renderer: F) {
        let mut scene = vello::Scene::new();
        renderer(&mut scene);
        self.renderer.render_to_texture(
            device,
            queue,
            &scene,
            &self.view,
            &vello::RenderParams {
                base_color: vello::peniko::Color::from_rgb8(0, 0, 0),
                antialiasing_method: vello::AaConfig::Msaa16,
                width: 1280,
                height: 720,
            }
        ).expect("Failed to render.");
    }

    pub fn render(&self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_pipeline(&self.draw_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}