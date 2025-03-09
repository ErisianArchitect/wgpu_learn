
use std::arch::x86_64;
use std::time::{Duration, Instant};

use glam::{vec2, vec3, vec4, Vec3};
use wgpu::ShaderStages;
use wgpu::{self, util::DeviceExt};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{DeviceEvent, Event, MouseButton};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::{event::WindowEvent, window::Window};

use crate::camera::Camera;
use crate::input::Input;
use crate::modeling::modeler::Modeler;
use crate::rendering::texture_array::TextureArrayBindGroup;
use crate::voxel::vertex::Vertex;
use crate::rendering::{
    texture_array::TextureArray,
    transforms::TransformsBindGroup,
};
use crate::voxel_fog::{Fog, FogBindGroup};

pub struct Settings {
    pub mouse_smoothing: bool,
}

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
    // Fog
    pub fog_bind_group: FogBindGroup,
    pub fog: Fog,
    // Camera
    pub camera: Camera,
    // Input State
    pub input: Input,
    pub settings: Settings,
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
                cube_sides_dir.join("stone_bricks.png"),
                cube_sides_dir.join("stone_bricks.png"),
                cube_sides_dir.join("stone_bricks.png"),
                cube_sides_dir.join("stone_bricks.png"),
                cube_sides_dir.join("stone_bricks.png"),
                // cube_sides_dir.join("pos_y.png"),
                cube_sides_dir.join("stone_bricks.png"),
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
        let view_proj_matrix = camera.projection_view_matrix();
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

        

        let fog = Fog::new(50.0, 250.0, vec4(0.0, 0.0, 0.0, 1.0));
        let fog_bind_group = FogBindGroup::new(&device);
        fog_bind_group.write_fog(&queue, &fog);


        // Include Shader
        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/voxel.wgsl"));
        // Render Pipeline Layout
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
                &transforms.bind_group_layout,
                &texture_array_bind_group.bind_group_layout,
                &fog_bind_group.bind_group_layout,
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

        let mut m = Modeler::new();
        m.texture_index(4, move |m| {
            // m.push_unit_quad();
            for y in 0..16 {
                for x in 0..16 {
                    let xf = x as f32;
                    let yf = y as f32;
                    m.translate(vec3(xf, 0.0, yf), move |m| {
                        m.push_unit_quad();
                    });
                }
            }
        });

        // println!("{:?}", &m.vertices);
        // println!("{:?}", &m.indices);
        // std::thread::sleep(Duration::from_secs(5));

        // m.vertices.extend_from_slice(Vertex::PLANE_VERTICES);
        // m.indices.extend_from_slice(Vertex::PLANE_INDICES);

        // Vertex Buffer
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(m.vertices.as_slice()),
            usage: wgpu::BufferUsages::VERTEX,
        });
        // Index Buffer
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(m.indices.as_slice()),
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
            num_indices: m.indices.len() as u32,
            texture_array,
            texture_array_bind_group,
            camera,
            transforms,
            fog_bind_group,
            fog,
            last_time: std::time::Instant::now(),
            input: Input::default(),
            settings: Settings {
                mouse_smoothing: true,
            },
        }
    }

    pub fn window_center(&self) -> PhysicalPosition<f64> {
        PhysicalPosition::new(
            self.size.width as f64 / 2.0,
            self.size.height as f64 / 2.0,
        )
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

    pub fn process_event(&mut self, event: &Event<()>) {
        match event {
            Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta }, .. } => {
                self.input.mouse_pos.delta.x += delta.0;
                self.input.mouse_pos.delta.y += delta.1;
                // self.window.set_cursor_position(self.window_center()).unwrap();
                // const MOUSE_SENSITIVITY: f64 = 0.00075;
                // let rot_y = -(delta.0 * MOUSE_SENSITIVITY);
                // let rot_x = -(delta.1 * MOUSE_SENSITIVITY);
                // self.camera.rotate(vec2(rot_x as f32, rot_y as f32));
            }
            _ => (),
        }
    }

    pub fn process_window_event(&mut self, _event: &WindowEvent) -> bool {
        match _event {
            WindowEvent::MouseInput { state, button, .. } => {
                self.input.set_mouse_state(*button, state.is_pressed());
            },
            WindowEvent::CursorMoved { position, .. } => {
                self.input.mouse_pos.current = *position;
            },
            WindowEvent::MouseWheel { device_id, delta, phase } => {
                match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => {
                        let diff = *y;
                        self.fog.start = self.fog.start + diff * 3.0;
                    },
                    winit::event::MouseScrollDelta::PixelDelta(physical_position) => todo!(),
                }
            }
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

    pub fn begin_frame(&mut self, _frame_index: u64) {
        self.input.begin_frame(self.settings.mouse_smoothing);
    }

    pub fn end_frame(&mut self, _frame_index: u64) {
        // let w = self.size.width as f64;
        // let h = self.size.height as f64;
        // let mid_x = w * 0.5;
        // let mid_y = h * 0.5;
        // let mid_pos = PhysicalPosition::new(mid_x, mid_y);
        // self.input.mouse_pos.current = mid_pos;
        // self.window.set_cursor_position(mid_pos).unwrap();
        self.input.end_frame();
    }

    pub fn begin_update(&mut self, _frame_index: u64) {

    }

    pub fn end_update(&mut self, _frame_index: u64) {

    }

    pub fn update(&mut self, frame_index: u64) {
        
        let elapsed = self.last_time.elapsed();
        let t = elapsed.as_secs_f32();

        if self.input.key_just_pressed(KeyCode::F11) {
            // self.window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
            if let Some(_) = self.window.fullscreen() {
                self.window.set_fullscreen(None);
            } else {
                self.window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
            }
        }

        // let rot = -15f32.to_radians() * t;
        // self.camera.rotate(vec2(0.0, rot));

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

        let mut total_movement = Vec3::ZERO;
        let mut moved = false;

        let w = self.input.key_pressed(KeyCode::KeyW);
        let a = self.input.key_pressed(KeyCode::KeyA);
        let s = self.input.key_pressed(KeyCode::KeyS);
        let d = self.input.key_pressed(KeyCode::KeyD);
        let r = self.input.key_pressed(KeyCode::KeyR);
        let f = self.input.key_pressed(KeyCode::KeyF);
        

        // Forward
        if w && !s {
            total_movement += Vec3::NEG_Z;
            moved = true;
            // self.camera.translate_rotated(Vec3::NEG_Z * t);
        } else if s && !w {
            total_movement += Vec3::Z;
            moved = true;
            // self.camera.translate_rotated(Vec3::Z * t);
        }
        // Leftward
        if a && !d {
            total_movement += Vec3::NEG_X;
            moved = true;
            // self.camera.translate_rotated(Vec3::NEG_X * t);
        } else if d && !a {
            total_movement += Vec3::X;
            moved = true;
            // self.camera.translate_rotated(Vec3::X * t);
        }
        // Rise
        if r && !f {
            total_movement += Vec3::Y;
            moved = true;
            // self.camera.translate_rotated(Vec3::Y * t);
        } else if f && !r {
            total_movement += Vec3::NEG_Y;
            moved = true;
            // self.camera.translate_rotated(Vec3::NEG_Y * t);
        }

        if moved {
            let movement = total_movement.normalize() * t * 4.0;
            self.camera.translate_planar(movement);
        }
        
        let mouse_pos = self.input.mouse_pos.current;
        let screen_pos = vec2(
            ((mouse_pos.x / self.size.width as f64) * 2.0 - 1.0) as f32,
            ((mouse_pos.y / self.size.height as f64) * 2.0 - 1.0) as f32,
        );
        let ray = self.camera.normalized_screen_to_ray(screen_pos);

        if self.input.key_just_pressed(KeyCode::KeyB) {
            println!("{:.5}, {:.5}", ray.dir.length(), ray.invert_dir().dir.length());
        }

        if self.input.mouse_pressed(MouseButton::Left) {
            // let new_pos = ray.point_on_ray(t);
            // self.camera.position = new_pos;
            self.camera.position = ray.point_on_ray(t * 0.25);
        }
        if self.input.mouse_pressed(MouseButton::Right) {
            // let ray = ray.invert_dir();
            // let new_pos = ray.point_on_ray(t);
            self.camera.position = ray.point_on_ray(t * -0.25);
        }
        self.texture_array.texel_to_uv(vec2(32.0, 32.0));
        if self.input.key_just_pressed(KeyCode::KeyC) {
            self.window.set_content_protected(true);
        }
        if self.input.key_just_pressed(KeyCode::KeyX) {
            self.window.set_content_protected(false);
        }

        // Mouse Move

        // Toggle Mouse Smoothing
        if self.input.key_just_pressed(KeyCode::KeyH) {
            self.settings.mouse_smoothing = !self.settings.mouse_smoothing;
            println!("Mouse Smoothing: {}", self.settings.mouse_smoothing);
        }

        // Change Smoothing Frame Count
        if self.input.key_just_pressed(KeyCode::ArrowUp) {
            let capacity = self.input.mouse_pos.delta_avg.capacity();
            if capacity < 30 {
                self.input.mouse_pos.delta_avg.set_capacity(capacity + 1);
                println!("Smoothing Capacity: {}", capacity + 1);
            }
        }
        if self.input.key_just_pressed(KeyCode::ArrowDown) {
            let capacity = self.input.mouse_pos.delta_avg.capacity();
            if capacity > 0 {
                self.input.mouse_pos.delta_avg.set_capacity(capacity - 1);
                println!("Smoothing Capacity: {}", capacity - 1);
            }
        }

        const MOUSE_SENSITIVITY: f64 = 0.00075;

        if self.input.mouse_just_pressed(MouseButton::Middle) {
            self.window.set_cursor_visible(true);
        }
        if self.input.mouse_just_released(MouseButton::Middle) {
            self.window.set_cursor_visible(false);
        }

        if !self.input.mouse_pressed(MouseButton::Middle) {
            let rot_y = -(self.input.mouse_pos.delta.x * MOUSE_SENSITIVITY);
            let rot_x = -(self.input.mouse_pos.delta.y * MOUSE_SENSITIVITY);
    
            self.camera.rotate(vec2(rot_x as f32, rot_y as f32));
            self.window.set_cursor_position(self.window_center()).unwrap();
        }

        if self.input.key_just_pressed(KeyCode::KeyT) {
            println!("FPS: {:.0}, FI: {frame_index}", 1.0 / t);
            println!("Rotation: {:.0}", self.camera.rotation.y.to_degrees());
        }
        // const MOUSE_SENSITIVITY: f32 = 0.05;
        // let mouse_offset = self.input.mouse_offset();
        // let mx = (mouse_offset.x as f32 * MOUSE_SENSITIVITY) * t;
        // let my = (mouse_offset.y as f32 * MOUSE_SENSITIVITY) * t;

        // self.camera.rotate(vec2(-my, -mx));

        if frame_index % 60 == 0 {
            println!("Frame: {frame_index}");
        }
        // let fps = 1.0 / t;
        // if fps < 55.0 {
        //     println!("FPS: {}", fps);
        // }

        self.last_time = std::time::Instant::now();
    }

    /// Called at the start of render() so that render resources can be initialized.
    fn render_init(&mut self) {
        // Update the view/projection matrix in the transform bind group buffer.
        self.transforms.write_view_projection(&self.queue, &self.camera.projection_view_matrix());
        self.transforms.write_camera_position(&self.queue, &self.camera.position);
        self.fog_bind_group.write_fog(&self.queue, &self.fog);
    }

    pub fn render(&mut self, _frame_index: u64) -> Result<Duration, wgpu::SurfaceError> {
        let start_time = Instant::now();
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
        render_pass.set_bind_group(2, &self.fog_bind_group.bind_group, &[]);

        // const LOCS: &'static [Vec3] = &[
        //     vec3(-1., 0., -1.), vec3(0., 0., -1.), vec3(1., 0., -1.),
        //     vec3(-1., 0., 0.), vec3(0., 0., 0.), vec3(1., 0., 0.),
        //     vec3(-1., 0., 1.), vec3(0., 0., 1.), vec3(1., 0., 1.),
        // ];

        // let mat1 = glam::Mat4::IDENTITY;
        // let mat2 = glam::Mat4::from_scale_rotation_translation(Vec3::ONE, Quat::IDENTITY, Vec3::X);
        // let mat2 = glam::Mat4::from_translation(Vec3::X);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        // for &loc in LOCS.iter() {
        //     let world = glam::Mat4::from_translation(loc);
        //     render_pass.set_push_constants(ShaderStages::VERTEX, 0, bytemuck::bytes_of(&world));
        //     render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        // }
        for z in -64..64 {
            for x in -64..64 {
                let world = glam::Mat4::from_translation(Vec3::new(x as f32 * 16.0, 0.0, z as f32 * 16.0));
                render_pass.set_push_constants(ShaderStages::VERTEX, 0, bytemuck::bytes_of(&world));
                render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
            }
        }

        // render_pass.set_push_constants(ShaderStages::VERTEX, 0, bytemuck::bytes_of(&mat2));
        // render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        // render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        // render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        drop(render_pass);
        self.queue.submit(std::iter::once(encoder.finish()));
        let time = start_time.elapsed();
        output.present();
        Ok(time)
    }
}