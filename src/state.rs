#![allow(unused)]
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::fmt::Write;

use gilrs::Gilrs;
use glam::{vec2, vec3, vec4, Vec3};
use wgpu::{MemoryHints, MultisampleState, ShaderStages, TextureFormat};
use wgpu::{self, util::DeviceExt};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{DeviceEvent, Event, MouseButton};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::{event::WindowEvent, window::Window};

use crate::animation::animtimer::AnimTimer;
use crate::camera::Camera;
use crate::input::Input;
use crate::math::average::{AverageBuffer, AvgBuffer};
use crate::modeling::modeler::Modeler;
use crate::rendering::raytrace::{AmbientLight, DirectionalLight, GpuMat3, GpuTransform, GpuVec3, Lighting, PrecomputedDirections, RaytraceChunk, Raytracer};
use crate::rendering::reticle::Reticle;
use crate::rendering::skybox::{Skybox, SkyboxTexturePaths};
use crate::rendering::texture_array::TextureArrayBindGroup;
use crate::voxel::vertex::Vertex;
use crate::rendering::{
    texture_array::TextureArray,
    transforms::TransformsBindGroup,
};
use crate::voxel_fog::{Fog, FogBindGroup};
use crate::FrameInfo;

use glyphon::{Attrs, Buffer, Cache, Color, FontSystem, Metrics, Resolution, SwashCache, TextArea, TextAtlas, TextRenderer, Viewport, Weight};

pub struct Settings {
    pub mouse_smoothing: bool,
    pub mouse_halting: bool,
}

pub struct TextRend {
    font_system: FontSystem,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
    front_buffer: Buffer,
    back_buffer: Buffer,
    cache: Cache,
    swash_cache: SwashCache,
}

pub struct StateAnimator {
    timer: AnimTimer,
    callback: Box<dyn Fn(&mut State<'_>, &AnimTimer) + 'static>,
}

impl StateAnimator {
    pub fn start<F: Fn(&mut State<'_>, &AnimTimer) + 'static>(duration: Duration, callback: F) -> Self {
        Self {
            timer: AnimTimer::start(duration),
            callback: Box::new(callback),
        }
    }

    /// Update the animation and return true when it is finished.
    /// 
    /// This function will call the inner callback regardless of whether or not the animation is finished.
    /// You should use the result of this function to determine when to stop calling this function.
    /// Think of it like this:
    /// ```rust, no_run
    /// let mut state: State<'_> = ...;
    /// let animator: StateAnimator = ...;
    /// let mut should_stop_animating = animator.is_finished();
    /// // ... later in loop
    /// if !should_stop_animating {
    ///     should_stop_animating = animator.update(&mut state);
    /// }
    /// 
    /// ```
    pub fn update(&self, state: &mut State<'_>) -> bool {
        (self.callback)(state, &self.timer);
        self.timer.is_finished()
    }
}

// pub struct Animation

const MOVE_SPEEDS: [f32; 7] = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0];

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
    // Transforms
    pub transforms: TransformsBindGroup,
    // Fog
    pub fog_bind_group: FogBindGroup,
    pub fog: Fog,
    // Camera
    pub camera: Camera,
    pub move_speed_index: usize,
    // Input State
    pub input: Input,
    pub gamepad: Gilrs,
    pub settings: Settings,
    pub text_rend: TextRend,
    pub locked: bool,
    pub animation: Option<StateAnimator>,
    // pub depth_stencil: wgpu::Texture,
    // pub depth_texture_view: wgpu::TextureView,
    // pub glyphon_pipeline: wgpu::RenderPipeline,
    pub raytracer: Raytracer,
    pub raytrace_timer: AverageBuffer<Duration>,
    pub rt_query_buffer: wgpu::Buffer,
    pub rt_query_read_buffer: wgpu::Buffer,
    pub rt_query_set: wgpu::QuerySet,
    pub reticle: Reticle,
    pub ortho: glam::Mat4,
}

impl<'a> State<'a> {
    pub async fn new(window: &'a Window) -> State<'a> {
        let size = window.inner_size();
        let aspect_ratio = size.width as f32 / size.height as f32;
        // Instance
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
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
                required_features: wgpu::Features::PUSH_CONSTANTS
                | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                | wgpu::Features::TIMESTAMP_QUERY
                | wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES,
                required_limits: limits,
                label: None,
                memory_hints: MemoryHints::Performance,
            },
            None
        ).await.unwrap();
        // adapter.request_device(
        //     &DeviceDescriptor {
        //         features: Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
        //         ..Default::default()
        //     },
        //     None,
        // ).await
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
        // Surface Caps/Format
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
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
        // Texture Array
        let cube_sides_dir = std::path::PathBuf::from("./assets/textures/cube_sides/");
        let texture_array = TextureArray::from_files(
            &device,
            &queue,
            &[
                cube_sides_dir.join("packed_dirt3.png"),
                cube_sides_dir.join("packed_dirt3.png"),
                cube_sides_dir.join("packed_dirt3.png"),
                cube_sides_dir.join("packed_dirt3.png"),
                cube_sides_dir.join("packed_dirt3.png"),
                cube_sides_dir.join("packed_dirt3.png"),
                // cube_sides_dir.join("pos_y.png"),
            ],
            Some("Debug Texture Array"),
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::AddressMode::Repeat,
            wgpu::AddressMode::Repeat,
            5,
        ).expect("Failed to load texture array.");
        // Texture Array Bind Group
        // let texture_array_bind_group = texture_array.bind_group(&device);
        // Transforms
        let transforms = TransformsBindGroup::new(&device);

        let skybox_dir = std::path::PathBuf::from("./assets/textures/skyboxes/complex/");
        let skybox = Skybox::new(
            &device,
            &queue,
            &config,
            Some("Skybox"),
            wgpu::TextureFormat::Rgba8UnormSrgb,
            // surface_format,
            &transforms,
            &SkyboxTexturePaths {
                top: skybox_dir.join("purp_top.png"),
                bottom: skybox_dir.join("purp_bottom.png"),
                left: skybox_dir.join("purp_left.png"),
                right: skybox_dir.join("purp_right.png"),
                front: skybox_dir.join("purp_front.png"),
                back: skybox_dir.join("purp_back.png"),
            }
        ).expect("Failed to load skybox.");
        
        // Camera
        let camera = Camera::from_look_to(
            Vec3::new(0.0, 16.0, 0.0),
            vec3(-1.0, 0.0, 1.0).normalize(),
            60f32.to_radians(),
            0.01,
            50000.0,
            size,
            skybox,
        );
        let view_proj_matrix = camera.projection_view_matrix();
        // transforms.write_world(&queue, &glam::Mat4::from_scale_rotation_translation(Vec3::ONE, Quat::IDENTITY, Vec3::ZERO));
        transforms.write_view_projection(&queue, &view_proj_matrix);
        

        

        let fog = Fog::new(40000.0, 50000.0, vec4(60.0, 60.0, 60.0, 0.0));
        let fog_bind_group = FogBindGroup::new(&device);
        fog_bind_group.write_fog(&queue, &fog);


        // Include Shader
        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/voxel.wgsl"));
        // Render Pipeline Layout
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
                &transforms.bind_group_layout,
                &texture_array.bind_group.bind_group_layout,
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
                entry_point: Some("vs_main"),
                buffers: &[
                    Vertex::desc()
                ],
                compilation_options: wgpu::PipelineCompilationOptions {
                    ..Default::default()
                },
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions {
                    ..Default::default()
                },
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
            cache: None,
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

        let text_rend = {
            let mut font_system = FontSystem::new();
            let cache = glyphon::Cache::new(&device);
            let mut text_atlas = TextAtlas::new(&device, &queue, &cache, surface_format);
            let text_renderer = TextRenderer::new(
                &mut text_atlas,
                &device,
                MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false
                },
                None,
            );

            let mut front_buffer = Buffer::new(&mut font_system, Metrics::new(48.0, 48.0));
            front_buffer.set_size(&mut font_system, Some(size.width as f32), Some(size.height as f32));
            let mut back_buffer = Buffer::new(&mut font_system, Metrics::new(48.0, 48.0));
            front_buffer.set_size(&mut font_system, Some(size.width as f32), Some(size.height as f32));

            TextRend {
                font_system,
                cache,
                text_atlas,
                text_renderer,
                front_buffer,
                back_buffer,
                swash_cache: SwashCache::new(),
            }
        };

        

        // Depth texture
        // let (depth_stencil, depth_texture_view) = {

        //     let depth_stencil = device.create_texture(&wgpu::TextureDescriptor {
        //         label: Some("Depth Texture"),
        //         size: wgpu::Extent3d {
        //             width: size.width,
        //             height: size.height,
        //             depth_or_array_layers: 1,
        //         },
        //         mip_level_count: 1,
        //         sample_count: 1,
        //         dimension: wgpu::TextureDimension::D2,
        //         format: wgpu::TextureFormat::Depth32Float,
        //         usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        //         view_formats: &[],
        //     });

        //     let depth_texture_view = depth_stencil.create_view(&wgpu::TextureViewDescriptor::default());

        //     (
        //         // TODO
        //         depth_stencil,
        //         depth_texture_view,
        //     )
        // };
        let mut chunk = RaytraceChunk::new();
        for z in 0..64 {
            for x in 0..64 {
                for y in 0..64 {
                    chunk.set(x, y, z, 1);
                }
            }
        }
        // for i in 1..16 {
        //     for z in 0+i..64-i {
        //         for x in 0+i..64-i {
        //             chunk.set(x, i, z, 1);
        //         }
        //     }
        // }
        // for y in 16..32 {
        //     for z in 30..34 {
        //         for x in 30..34 {
        //             chunk.set(x, y, z, 1);
        //         }
        //     }
        // }
        // TODO: Uncomment this section.
        // for z in 0..64 {
        //     for x in 0..64 {
        //         chunk.set(x, 0, z, 1);
        //     }
        // }
        // let sy = 1;
        // for z in 0..4 {
        //     for x in 0..4 {
        //         for y in sy..sy+16 {
        //             chunk.set(x, y, z, 1);
        //         }
        //     }
        // }
        let mut raytracer = Raytracer::new(&device, &queue, &camera, Some(chunk), &Lighting {
            directional: DirectionalLight {
                // color: vec3(0.9568627450980393, 0.9137254901960784, 0.6078431372549019),
                color: vec3(1.0, 1.0, 1.0),
                direction: vec3(1.0, -4.0, 2.0).normalize(),
                intensity: 1.0,
                evening_intensity: 10.0 / 255.0,
                shadow: 0.2,
                active: true,
            },
            ambient: AmbientLight {
                color: Vec3::ONE,
                intensity: 0.1,
                active: true,
            }
        });
        let raytrace_timer = AverageBuffer::<Duration>::new(100, None);
        let reticle = match Reticle::new(&device, &queue, "assets/textures/reticles/crosshair118.png", &config) {
            Ok(reticle) => reticle,
            Err(err) => panic!("Error Creating Reticle: {err}"),
        };

        let ortho = glam::Mat4::orthographic_rh(0.0, size.width as f32, size.height as f32, 0.0, 0.0, 100.0);

        let rt_query_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Raytrace Timestamp Buffer"),
            size: 16,
            mapped_at_creation: false,
            usage:  wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
        });

        let rt_query_read_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Raytrace Timestamp Read Buffer"),
            size: 16,
            mapped_at_creation: false,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        });

        let rt_query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("Raytrace Query Set"),
            count: 2,
            ty: wgpu::QueryType::Timestamp,
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
            camera,
            move_speed_index: 4,
            transforms,
            fog_bind_group,
            fog,
            last_time: std::time::Instant::now(),
            input: Input::default(),
            gamepad: Gilrs::new().expect("Failed to create gamepad."),
            settings: Settings {
                mouse_smoothing: false,
                mouse_halting: false,
            },
            text_rend,
            locked: false,
            animation: None,
            // depth_stencil,
            // depth_texture_view,
            raytracer,
            raytrace_timer,
            rt_query_buffer,
            rt_query_read_buffer,
            rt_query_set,
            reticle,
            ortho,
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
            // self.camera.aspect_ratio = new_size.width as f32 / new_size.height as f32;
            self.camera.resize(new_size);
            self.ortho = glam::Mat4::orthographic_rh(0.0, new_size.width as f32, new_size.height as f32, 0.0, 0.0, 100.0);
            self.reticle.write_dimensions(&self.queue, new_size.width, new_size.height);
            self.reticle.write_ortho(&self.queue, &self.ortho);
            // self.text_rend.buffer.set_size(&mut self.text_rend.font_system, Some(new_size.width as f32), Some(new_size.height as f32));
        }
    }

    pub fn focus_changed(&mut self, _focus: bool) {

    }

    pub fn close_requested(&mut self) -> bool {
        
        true
    }

    pub fn process_gamepad_event(&mut self, event: &gilrs::Event) {
        match event.event {
            gilrs::EventType::ButtonPressed(button, code) => {
            },
            gilrs::EventType::ButtonRepeated(button, code) => {

            },
            gilrs::EventType::ButtonReleased(button, code) => {

            },
            gilrs::EventType::ButtonChanged(button, t, code) => {
                match button {
                    gilrs::Button::LeftTrigger => {
                        
                    },
                    gilrs::Button::LeftTrigger2 => {
                        
                    },
                    gilrs::Button::RightTrigger => {
                    },
                    gilrs::Button::RightTrigger2 => {
                        
                        println!("{t:.4}")
                    },
                    _ => (),
                }
            },
            gilrs::EventType::AxisChanged(axis, t, code) => {
                
            },
            gilrs::EventType::Connected => {
                
            },
            gilrs::EventType::Disconnected => {
                
            },
            gilrs::EventType::Dropped => {
                
            },
            gilrs::EventType::ForceFeedbackEffectCompleted => {
                
            },
            _ => (),
        }
    }

    pub fn process_event(&mut self, event: &Event<()>) {
        match event {
            Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta }, .. } => {
                self.input.mouse_pos.delta.x += delta.0;
                self.input.mouse_pos.delta.y += delta.1;
                self.input.mouse_pos.live_mouse.set_target(delta.0, delta.1);
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
                // self.input.mouse_pos.live_mouse.set_target(position.x, position.y);
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
            },
            _=>(),
        }
        false
    }

    pub fn begin_frame(&mut self, frame: &FrameInfo) {
        self.input.begin_frame(&self.settings, frame);
    }

    pub fn end_frame(&mut self, frame: &FrameInfo) {
        // let w = self.size.width as f64;
        // let h = self.size.height as f64;
        // let mid_x = w * 0.5;
        // let mid_y = h * 0.5;
        // let mid_pos = PhysicalPosition::new(mid_x, mid_y);
        // self.input.mouse_pos.current = mid_pos;
        // self.window.set_cursor_position(mid_pos).unwrap();
        self.input.end_frame();
    }

    pub fn begin_update(&mut self, frame: &FrameInfo) {

    }

    pub fn end_update(&mut self, frame: &FrameInfo) {

    }

    pub fn update(&mut self, frame: &FrameInfo) {
        
        let elapsed = self.last_time.elapsed();
        let t = frame.delta_time.as_secs_f32();

        if self.input.key_just_pressed(KeyCode::F11) {
            // self.window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
            if let Some(_) = self.window.fullscreen() {
                self.window.set_fullscreen(None);
            } else {
                self.window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
            }
        }

        let mut total_movement = Vec3::ZERO;
        let mut moved = false;
        let ctrl = self.input.key_pressed(KeyCode::ControlLeft) || self.input.key_pressed(KeyCode::ControlRight);
        let alt_l = self.input.key_pressed(KeyCode::AltLeft);
        let w = self.input.key_pressed(KeyCode::KeyW);
        let s = self.input.key_pressed(KeyCode::KeyS);

        let a = self.input.key_pressed(KeyCode::KeyA);
        let d = self.input.key_pressed(KeyCode::KeyD);

        let r = self.input.key_pressed(KeyCode::KeyR);
        let f = self.input.key_pressed(KeyCode::KeyF);

        let tk = self.input.key_pressed(KeyCode::KeyT);
        let g = self.input.key_pressed(KeyCode::KeyG);

        let d2 = self.input.key_pressed(KeyCode::Digit2);
        let x = self.input.key_pressed(KeyCode::KeyX);
        
        let move_speed = MOVE_SPEEDS[self.move_speed_index];

        let move_multiplier = if self.input.key_pressed(KeyCode::ShiftLeft) {
            4.0 * move_speed
        } else if alt_l {
            0.25 * move_speed
        } else {
            MOVE_SPEEDS[self.move_speed_index]
        };

        // Forward (Planar)
        if w && !s {
            total_movement += Vec3::NEG_Z;
            moved = true;
            // self.camera.translate_rotated(Vec3::NEG_Z * t);
        } else if s && !w && !ctrl { // Backward (Planar)
            total_movement += Vec3::Z;
            moved = true;
            // self.camera.translate_rotated(Vec3::Z * t);
        }

        // Forward (Free)
        if d2 && !x {
            self.camera.position += self.camera.forward() * t * move_multiplier;
            moved = true;
            // self.camera.translate_rotated(Vec3::Y * t);
        } else if x && !d2 { // Backward (Free)
            self.camera.position += self.camera.backward() * t * move_multiplier;
            moved = true;
            // self.camera.translate_rotated(Vec3::NEG_Y * t);
        }

        // Rise
        if r && !f {
            total_movement += Vec3::Y;
            moved = true;
            // self.camera.translate_rotated(Vec3::Y * t);
        } else if f && !r { // Fall
            total_movement += Vec3::NEG_Y;
            moved = true;
            // self.camera.translate_rotated(Vec3::NEG_Y * t);
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
        

        if moved {
            let movement = total_movement.normalize() * t * move_multiplier;
            self.camera.translate_planar(movement);
            self.animation.take();
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

        if self.input.key_pressed(KeyCode::KeyQ) {
            self.raytracer.gpu_lighting.set_directional_direction(&self.queue, ray.dir.into());
        }

        if self.input.mouse_just_pressed(MouseButton::Left) {
            // let new_pos = ray.point_on_ray(t);
            // self.camera.position = new_pos;
            // self.camera.position = ray.point_on_ray(t * 0.25).into();
            if let Some(hit) = self.raytracer.chunk.raycast(ray, 200.0) {
                let cell = hit.get_hit_cell();
                self.raytracer.chunk.set(cell.x, cell.y, cell.z, 1);
            }
        }
        if self.input.mouse_just_pressed(MouseButton::Right) {
            // let ray = ray.invert_dir();
            // let new_pos = ray.point_on_ray(t);
            if let Some(hit) = self.raytracer.chunk.raycast(ray, 200.0) {
                let cell = hit.coord;
                self.raytracer.chunk.set(cell.x, cell.y, cell.z, 0);
            }
        }
        let chunk_path = "./sandbox_files/chunk.dat";
        // self.texture_array.texel_to_uv(vec2(32.0, 32.0));
        if self.input.key_just_pressed(KeyCode::KeyS) && ctrl {
            self.raytracer.chunk.save(chunk_path).expect("Failed to save chunk.");
            println!("Saved chunk to file \"{chunk_path}\".");
        }
        if self.input.key_just_pressed(KeyCode::KeyL) {
            let load_start = Instant::now();
            match self.raytracer.chunk.load(chunk_path) {
                Ok(()) => {
                    let load_elapsed = load_start.elapsed();
                    println!("Loaded chunk from file \"{chunk_path}\" in {load_elapsed:.2?}");
                }
                Err(err) => {
                    eprintln!("Failed to load file: \"{chunk_path}\"");
                    eprintln!("Error: {err:?}");
                }
            }
        }

        if self.input.key_just_pressed(KeyCode::Tab) {
            self.locked = !self.locked;
            if self.locked {
                self.window.set_cursor_visible(false);
            } else {
                self.window.set_cursor_visible(true);
            }
        }

        if self.input.key_pressed(KeyCode::KeyE) {
            self.camera.position += self.camera.forward() * t * move_multiplier;
        }

        if self.input.key_just_pressed(KeyCode::ArrowRight) {
            // self.move_speed_index = (self.move_speed_index + 1) % MOVE_SPEEDS.len();
            self.move_speed_index = (self.move_speed_index + 1).min(MOVE_SPEEDS.len() - 1);
            // let start = self.camera.position;
            // let mut end = self.camera.position + self.camera.right() * 4.0;
            // self.animation.replace(StateAnimator::start(Duration::from_secs(1), move |state, anim| {
            //     use crate::animation::tween;
            //     let pos = start.lerp(end, tween::f32::quartic_in_out(anim.alpha_f32()));
            //     state.camera.position = pos;
            // }));
        } else if self.input.key_just_pressed(KeyCode::ArrowLeft) {
            // self.move_speed_index = (self.move_speed_index + MOVE_SPEEDS.len() - 1) % MOVE_SPEEDS.len();
            self.move_speed_index = self.move_speed_index.saturating_sub(1);
            // let start = self.camera.position;
            // let mut end = self.camera.position + self.camera.left() * 4.0;
            // self.animation.replace(StateAnimator::start(Duration::from_secs(1), move |state, anim| {
            //     use crate::animation::tween;
            //     let pos = start.lerp(end, tween::f32::quartic_in_out(anim.alpha_f32()));
            //     state.camera.position = pos;
            // }));
        }

        if self.input.key_just_pressed(KeyCode::KeyY) {
            let start = self.camera.position;
            let mut end = vec3(64.0*16.0, 1.0, 64.0*16.0);
            self.animation.replace(StateAnimator::start(Duration::from_secs(10), move |state, anim| {
                use crate::animation::tween;
                let pos = start.lerp(end, tween::f32::quartic_in_out(anim.alpha_f32()));
                state.camera.position = pos;
            }));
        }

        // Mouse Move

        // Toggle Mouse Smoothing
        if self.input.key_just_pressed(KeyCode::KeyH) {
            self.settings.mouse_smoothing = !self.settings.mouse_smoothing;
        }
        if self.input.key_just_pressed(KeyCode::KeyJ) {
            self.settings.mouse_halting = !self.settings.mouse_halting;
        }

        // Change Smoothing Frame Count
        if self.input.key_just_pressed(KeyCode::ArrowUp) {
            let capacity = self.input.mouse_pos.delta_avg.capacity();
            if capacity < 30 {
                self.input.mouse_pos.delta_avg.set_capacity(capacity + 1);
            }
        }
        if self.input.key_just_pressed(KeyCode::ArrowDown) {
            let capacity = self.input.mouse_pos.delta_avg.capacity();
            if capacity > 1 {
                self.input.mouse_pos.delta_avg.set_capacity(capacity - 1);
            }
        }

        const MOUSE_SENSITIVITY: f64 = 0.00075 * 2.5;

        // if !self.locked && self.input.mouse_just_pressed(MouseButton::Middle) {
        //     self.window.set_cursor_visible(true);
        // }
        // if !self.locked && self.input.mouse_just_released(MouseButton::Middle) {
        //     self.window.set_cursor_visible(false);
        // }

        if self.input.key_just_pressed(KeyCode::Digit4) {
            println!("{:?}", self.input.mouse_pos.live_mouse.velocity());
        }
        let middle_pressed = self.input.mouse_pressed(MouseButton::Middle);
        if self.locked || middle_pressed {
            // let rot_y = -(self.input.mouse_pos.live_mouse.velocity().0 * MOUSE_SENSITIVITY);
            // let rot_x = -(self.input.mouse_pos.live_mouse.velocity().1 * MOUSE_SENSITIVITY);
            let rot_y = -(self.input.mouse_pos.delta.x * MOUSE_SENSITIVITY);
            let rot_x = -(self.input.mouse_pos.delta.y * MOUSE_SENSITIVITY);
            
            self.camera.rotate(vec2(rot_x as f32, rot_y as f32));
            if !middle_pressed {
                self.window.set_cursor_position(self.window_center()).unwrap();
            }
        }

        if let Some(mut anim) = self.animation.take() {
            if !anim.update(self) {
                self.animation = Some(anim);
            }
        }

        // if self.input.key_just_pressed(KeyCode::KeyT) {
        //     println!("FPS: {:.0}, FI: {}", 1.0 / t, frame.index);
        //     println!("Rotation: {:.0}", self.camera.rotation.y.to_degrees());
        // }
        // const MOUSE_SENSITIVITY: f32 = 0.05;
        // let mouse_offset = self.input.mouse_offset();
        // let mx = (mouse_offset.x as f32 * MOUSE_SENSITIVITY) * t;
        // let my = (mouse_offset.y as f32 * MOUSE_SENSITIVITY) * t;

        // self.camera.rotate(vec2(-my, -mx));
        // let fps = 1.0 / t;
        // if fps < 55.0 {
        //     println!("FPS: {}", fps);
        // }

        self.raytracer.write_camera_transform(GpuTransform::new(
            GpuMat3::new(self.camera.rotation_matrix()),
            GpuVec3::from_vec3(self.camera.position),
        ), &self.queue);
        self.raytracer.write_chunk(&self.queue);

        self.last_time = std::time::Instant::now();
    }

    /// Called at the start of render() so that render resources can be initialized.
    fn begin_render(&mut self) {
        // Update the view/projection matrix in the transform bind group buffer.
        self.transforms.write_view_projection(&self.queue, &self.camera.projection_view_matrix());
        self.transforms.write_camera_position(&self.queue, &self.camera.position);
        self.fog_bind_group.write_fog(&self.queue, &self.fog);
    }

    pub fn render(&mut self, frame: &FrameInfo) -> Result<Duration, wgpu::SurfaceError> {
        let start_time = Instant::now();
        self.begin_render();

        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        // let raytrace_start = Instant::now();
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Compute Encoder"),
        });

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Render Compute Pass"),
            timestamp_writes: None,
        });
        
        self.raytracer.compute(&mut compute_pass, Some(&self.rt_query_set));
        
        drop(compute_pass);
        encoder.resolve_query_set(&self.rt_query_set, 0..2, &self.rt_query_buffer, 0);
        encoder.copy_buffer_to_buffer(&self.rt_query_buffer, 0, &self.rt_query_read_buffer, 0, 16);
        self.queue.submit(Some(encoder.finish()));
        // let raytrace_elapsed = raytrace_start.elapsed();
        // self.raytrace_timer.push(raytrace_elapsed);

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
                            r: 0.0, g: 0.0, b: 0.0, a: 1.0
                        }),
                        store: wgpu::StoreOp::Store
                    }
            })],
            // depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            //     view: &self.depth_texture_view,
            //     depth_ops: Some(wgpu::Operations {
            //         load: wgpu::LoadOp::Clear(1.0),
            //         store: wgpu::StoreOp::Store,
            //     }),
            //     stencil_ops: None,
            // }),
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None
        });

        // increment
        // decrement
        // render_pass.set_pipeline(&self.render_pipeline);
        // render_pass.set_bind_group(0, &self.transforms.bind_group, &[]);
        // render_pass.set_bind_group(1, &self.texture_array.bind_group.bind_group, &[]);
        // render_pass.set_bind_group(2, &self.fog_bind_group.bind_group, &[]);

        // const LOCS: &'static [Vec3] = &[
        //     vec3(-1., 0., -1.), vec3(0., 0., -1.), vec3(1., 0., -1.),
        //     vec3(-1., 0., 0.), vec3(0., 0., 0.), vec3(1., 0., 0.),
        //     vec3(-1., 0., 1.), vec3(0., 0., 1.), vec3(1., 0., 1.),
        // ];

        // let mat1 = glam::Mat4::IDENTITY;
        // let mat2 = glam::Mat4::from_scale_rotation_translation(Vec3::ONE, Quat::IDENTITY, Vec3::X);
        // let mat2 = glam::Mat4::from_translation(Vec3::X);
        // render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        // render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        // for &loc in LOCS.iter() {
        //     let world = glam::Mat4::from_translation(loc);
        //     render_pass.set_push_constants(ShaderStages::VERTEX, 0, bytemuck::bytes_of(&world));
        //     render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        // }
        // for z in -64..64 {
        //     for x in -64..64 {
        //         let world = glam::Mat4::from_translation(Vec3::new(x as f32 * 16.0, 0.0, z as f32 * 16.0));
        //         render_pass.set_push_constants(ShaderStages::VERTEX, 0, bytemuck::bytes_of(&world));
        //         render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        //     }
        // }

        self.camera.render(&mut render_pass, &self.transforms);
        self.raytracer.render(&mut render_pass);

        let avg_rt_time = self.raytrace_timer.average();
    

        if self.locked {
            self.reticle.render(&mut render_pass);
        }
        
        // ██████████████████
        // █                █
        // █ Text Rendering █
        // █                █
        // ██████████████████
        {

            let mut viewport = Viewport::new(&self.device, &self.text_rend.cache);
            viewport.update(&self.queue, Resolution { width: self.size.width, height: self.size.height });

            let mut render_text = String::new();

            writeln!(render_text, "Frame Index: {}", frame.index);
            writeln!(render_text, "FPS: {:.0}", frame.fps);
            writeln!(render_text, "Raytrace Time: {avg_rt_time:.3?}");
            if self.settings.mouse_smoothing {
                writeln!(render_text, "Mouse Smoothing: {}", self.input.mouse_pos.delta_avg.capacity());
                writeln!(render_text, "Mouse Halting: {}", self.settings.mouse_halting);
            } else {
                writeln!(render_text, "Mouse Smoothing: Off");
            }
            writeln!(render_text, "Animating: {}", self.animation.is_some());
            writeln!(render_text, "Move Speed: {:.2}", MOVE_SPEEDS[self.move_speed_index]);

            self.text_rend.back_buffer.set_text(
                &mut self.text_rend.font_system,
                &render_text,
                Attrs::new()
                    // .color(Color::rgb(255, 255, 255))
                    ,
                glyphon::Shaping::Advanced,
            );
            let mut back_text = TextArea {
                bounds: glyphon::TextBounds { left: 0, top: 0, right: self.size.width as i32, bottom: self.size.height as i32 },
                buffer: &self.text_rend.back_buffer,
                left: 10.0,
                top: 10.0,
                scale: 1.0,
                default_color: Color::rgb(50, 50, 50),
                custom_glyphs: &[]
            };

            self.text_rend.front_buffer.set_text(
                &mut self.text_rend.font_system,
                &render_text,
                Attrs::new()
                    .color(Color::rgb(200, 200, 200))
                    ,
                glyphon::Shaping::Advanced,
            );
            let mut front_text = TextArea {
                bounds: glyphon::TextBounds { left: 0, top: 0, right: self.size.width as i32, bottom: self.size.height as i32 },
                buffer: &self.text_rend.front_buffer,
                left: 8.0,
                top: 9.0,
                scale: 1.0,
                default_color: Color::rgb(0, 0, 0),
                custom_glyphs: &[]
            };

            self.text_rend.text_renderer.prepare(&self.device, &self.queue, &mut self.text_rend.font_system, &mut self.text_rend.text_atlas, &viewport, [front_text, back_text], &mut self.text_rend.swash_cache).expect("Failed.");
            self.text_rend.text_renderer.render(&self.text_rend.text_atlas, &viewport, &mut render_pass).expect("Failed to render text.");
        }

        // render_pass.set_push_constants(ShaderStages::VERTEX, 0, bytemuck::bytes_of(&mat2));
        // render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        // render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        // render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        drop(render_pass);
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        let rt_ts_slice = self.rt_query_read_buffer.slice(..);
        let finished = Arc::new(AtomicBool::new(false));
        let finished_clone = Arc::clone(&finished);
        rt_ts_slice.map_async(wgpu::MapMode::Read, move |result| {
            if let Err(e) = result {
                panic!("Failed to map buffer: {e:?}");
            } else {
                finished_clone.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        });

        while !finished.load(std::sync::atomic::Ordering::Relaxed) {
            self.device.poll(wgpu::Maintain::Wait);
        }
        {
            let rt_ts_data = rt_ts_slice.get_mapped_range();
            let timestamps: &[u64] = bytemuck::cast_slice(&rt_ts_data);
            let ticks = timestamps[1] - timestamps[0];
            let time_ns = ticks as f64 * self.queue.get_timestamp_period() as f64;
            let rt_compute_time = Duration::from_nanos(time_ns as u64);
            self.raytrace_timer.push(rt_compute_time);
        }
        self.rt_query_read_buffer.unmap();
        let time = start_time.elapsed();
        Ok(time)
    }
}