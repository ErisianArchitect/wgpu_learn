
use glam::{vec2, vec3, Vec3};
use wgpu::ShaderStages;
use wgpu::{self, util::DeviceExt};
use winit::event::MouseButton;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::{event::WindowEvent, window::Window};

use crate::camera::Camera;
use crate::input::Input;
use crate::rendering::texture_array::TextureArrayBindGroup;
use crate::voxel::vertex::Vertex;
use crate::rendering::{
    texture_array::TextureArray,
    transforms::TransformsBindGroup,
};

pub struct State<'a> {
    pub surface: wgpu::Surface<'a>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    // The window must be declared after the surface so
    // it gets dropped after it as the surface contains
    // unsafe references to the window's resources.
    pub window: &'a Window,
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub last_time: std::time::Instant,
    // Texture Array
    pub texture_array: TextureArray,
    pub texture_array_bind_group: TextureArrayBindGroup,
    // Transforms
    pub transforms: TransformsBindGroup,
    // Camera
    pub camera: Camera,
    // Input State
    pub input: Input,
}

impl<'a> State<'a> {
    pub async fn new(window: &'a Window) -> State<'a> {
        let size = window.inner_size();
        let aspect_ratio = size.width as f32 / size.height as f32;
        // Instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        // Surface
        let surface = instance.create_surface(window).unwrap();
        // Adapter
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();
        let mut limits = wgpu::Limits {
            max_push_constant_size: 128,
            ..Default::default()
        };
        limits.max_push_constant_size = 256;
        // Device and Queue
        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::PUSH_CONSTANTS,
                required_limits: limits,
                label: None,
            },
            None
        ).await.unwrap();
        #[cfg(debug_assertions)]
        {
            match adapter.get_info().backend {
                wgpu::Backend::Vulkan => println!("Vulkan backend."),
                wgpu::Backend::Metal => println!("Metal backend. (wtf? This is running on a mac?)"),
                wgpu::Backend::Dx12 => println!("Dx12 backend."),
                wgpu::Backend::Gl => println!("Gl backend."),
                wgpu::Backend::BrowserWebGpu => println!("BrowserWebGpu backend."),
                _ => {}
            }
            let features = adapter.features();
            if features.contains(wgpu::Features::PUSH_CONSTANTS) {
                println!("Push constants are supported.");
            } else {
                println!("Push constants are not supported.");
            }
            let limits = device.limits();
            println!("Push constant size limit: {}", limits.max_push_constant_size);
        }
        // Texture Array
        let cube_sides_dir = std::path::PathBuf::from("./assets/textures/cube_sides/");
        let texture_array = TextureArray::from_files(
            &device,
            &queue,
            &[
                cube_sides_dir.join("neg_x.png"),
                cube_sides_dir.join("neg_y.png"),
                cube_sides_dir.join("neg_z.png"),
                cube_sides_dir.join("pos_x.png"),
                cube_sides_dir.join("pos_y.png"),
                cube_sides_dir.join("pos_z.png"),
            ],
            Some("Debug Texture Array"),
            wgpu::TextureFormat::Rgba8UnormSrgb,
        ).expect("Failed to load texture array.");
        // Texture Array Bind Group
        let texture_array_bind_group = texture_array.bind_group(&device);
        // Transforms
        let transforms = TransformsBindGroup::new(&device);
        // Camera
        let camera = Camera::from_look_at(Vec3::new(0.0, 1.0, 0.0), Vec3::NEG_Z, 45f32.to_radians(), aspect_ratio, 0.01, 1000.0);
        let view_proj_matrix = camera.view_projection_matrix();
        // transforms.write_world(&queue, &glam::Mat4::from_scale_rotation_translation(Vec3::ONE, Quat::IDENTITY, Vec3::ZERO));
        transforms.write_view_projection(&queue, &view_proj_matrix);
        // Surface Caps/Format
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        // Config
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            // enable vsync: (PresentMode::Fifo)
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        // Include Shader
        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/voxel.wgsl"));
        // Render Pipeline Layout
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
                &transforms.bind_group_layout,
                &texture_array_bind_group.bind_group_layout,
            ],
            push_constant_ranges: &[wgpu::PushConstantRange {
                range: 0..64,
                stages: wgpu::ShaderStages::VERTEX,
            }],
        });
        // Render Pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                    Vertex::desc()
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });
        // Vertex Buffer
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(Vertex::PLANE_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        // Index Buffer
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(Vertex::PLANE_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        // return
        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices: Vertex::PLANE_INDICES.len() as u32,
            texture_array,
            texture_array_bind_group,
            camera,
            transforms,
            last_time: std::time::Instant::now(),
            input: Input::default(),
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            self.camera.aspect_ratio = new_size.width as f32 / new_size.height as f32;
        }
    }

    pub fn focus_changed(&mut self, _focus: bool) {

    }

    pub fn close_requested(&mut self) -> bool {
        
        true
    }

    pub fn input(&mut self, _event: &WindowEvent) -> bool {
        match _event {
            WindowEvent::MouseInput { state, button, .. } => {
                self.input.set_mouse_state(*button, state.is_pressed());
            },
            WindowEvent::CursorMoved { position, .. } => {
                self.input.mouse_pos.current = *position;
            },
            WindowEvent::KeyboardInput { event, .. } => {
                if !event.repeat {
                    match event.physical_key {
                        PhysicalKey::Code(key) => {
                            self.input.set_key_state(key, event.state.is_pressed());
                        }
                        _ => (),
                    }
                }
                // if !event.repeat && event.state.is_pressed() {
                //     match event.physical_key {
                //         PhysicalKey::Code(KeyCode::Digit1) => {
                //             self.camera.look_at(Vertex::PLANE_VERTICES[0].position);
                //         },
                //         PhysicalKey::Code(KeyCode::Digit2) => {
                //             self.camera.look_at(Vertex::PLANE_VERTICES[1].position);
                //         },
                //         PhysicalKey::Code(KeyCode::Digit3) => {
                //             self.camera.look_at(Vertex::PLANE_VERTICES[2].position);
                //         },
                //         PhysicalKey::Code(KeyCode::Digit4) => {
                //             self.camera.look_at(Vertex::PLANE_VERTICES[3].position);
                //         },
                //         PhysicalKey::Code(KeyCode::Digit5) => {
                //             self.camera.look_at(Vec3::X);
                //         },
                //         _=>(),
                //     }
                // }
            },
            _=>(),
        }
        false
    }

    pub fn update_begin(&mut self) {

    }

    pub fn update(&mut self, frame_index: u64) {
        
        let elapsed = self.last_time.elapsed();
        let t = elapsed.as_secs_f32();

        let rot = -15f32.to_radians() * t;
        self.camera.rotate(vec2(0.0, rot));

        if self.input.key_just_pressed(KeyCode::Digit1) {
            self.camera.look_at(Vertex::PLANE_VERTICES[0].position);
        }
        if self.input.key_just_pressed(KeyCode::Digit2) {
            self.camera.look_at(Vertex::PLANE_VERTICES[1].position);
        }
        if self.input.key_just_pressed(KeyCode::Digit3) {
            self.camera.look_at(Vertex::PLANE_VERTICES[2].position);
        }
        if self.input.key_just_pressed(KeyCode::Digit4) {
            self.camera.look_at(Vertex::PLANE_VERTICES[3].position);
        }
        if self.input.key_pressed(KeyCode::Digit5) {
            self.camera.look_at(Vec3::X);
        }
        if self.input.mouse_pressed(MouseButton::Left) {
            println!("Left {frame_index}");
        }
        if self.input.mouse_just_pressed(MouseButton::Right) {
            println!("Right");
        }

        let mut total_movement = Vec3::ZERO;
        let mut moved = false;

        // Forward
        if self.input.key_pressed(KeyCode::KeyW) {
            total_movement += Vec3::NEG_Z;
            moved = true;
            // self.camera.translate_rotated(Vec3::NEG_Z * t);
        }
        // Leftward
        if self.input.key_pressed(KeyCode::KeyA) {
            total_movement += Vec3::NEG_X;
            moved = true;
            // self.camera.translate_rotated(Vec3::NEG_X * t);
        }
        // Backward
        if self.input.key_pressed(KeyCode::KeyS) {
            total_movement += Vec3::Z;
            moved = true;
            // self.camera.translate_rotated(Vec3::Z * t);
        }
        // Rightward
        if self.input.key_pressed(KeyCode::KeyD) {
            total_movement += Vec3::X;
            moved = true;
            // self.camera.translate_rotated(Vec3::X * t);
        }
        // Rise
        if self.input.key_pressed(KeyCode::KeyR) {
            total_movement += Vec3::Y;
            moved = true;
            // self.camera.translate_rotated(Vec3::Y * t);
        }
        // Fall
        if self.input.key_pressed(KeyCode::KeyF) {
            total_movement += Vec3::NEG_Y;
            moved = true;
            // self.camera.translate_rotated(Vec3::NEG_Y * t);
        }

        if moved {
            self.camera.translate_rotated(total_movement * t);
        }

        if self.input.key_just_pressed(KeyCode::KeyT) {
            println!("FPS: {:.0}, FI: {frame_index}", 1.0 / t);
            println!("Rotation: {:.0}", self.camera.rotation.y.to_degrees());
        }

        if frame_index % 60 == 0 {
            println!("Frame: {frame_index}");
        }

        self.last_time = std::time::Instant::now();
        self.update_finish();
    }

    pub fn update_finish(&mut self) {
        self.input.push_back();
    }

    /// Called at the start of render() so that render resources can be initialized.
    fn render_init(&mut self) {
        // Update the view/projection matrix in the transform bind group buffer.
        self.transforms.write_view_projection(&self.queue, &self.camera.view_projection_matrix());
    }

    pub fn render(&mut self, frame_index: u64) -> Result<(), wgpu::SurfaceError> {
        self.render_init();

        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder")
        });
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1, g: 0.2, b: 0.3, a: 1.0
                        }),
                        store: wgpu::StoreOp::Store
                    }
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None
        });
        // increment
        // decrement
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.transforms.bind_group, &[]);
        render_pass.set_bind_group(1, &self.texture_array_bind_group.bind_group, &[]);

        const LOCS: &'static [Vec3] = &[
            vec3(-1., 0., -1.), vec3(0., 0., -1.), vec3(1., 0., -1.),
            vec3(-1., 0., 0.), vec3(0., 0., 0.), vec3(1., 0., 0.),
            vec3(-1., 0., 1.), vec3(0., 0., 1.), vec3(1., 0., 1.),
        ];

        // let mat1 = glam::Mat4::IDENTITY;
        // let mat2 = glam::Mat4::from_scale_rotation_translation(Vec3::ONE, Quat::IDENTITY, Vec3::X);
        // let mat2 = glam::Mat4::from_translation(Vec3::X);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        for &loc in LOCS.iter() {
            let world = glam::Mat4::from_translation(loc);
            render_pass.set_push_constants(ShaderStages::VERTEX, 0, bytemuck::bytes_of(&world));
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        // render_pass.set_push_constants(ShaderStages::VERTEX, 0, bytemuck::bytes_of(&mat2));
        // render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        // render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        // render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        drop(render_pass);
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}