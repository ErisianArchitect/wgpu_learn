use glam::*;
use winit::dpi::{PhysicalSize, Size};
use wgpu::*;
use glyphon::{Attrs, Buffer, Cache, Color, FontSystem, Metrics, Resolution, SwashCache, TextArea, TextAtlas, TextRenderer, Viewport};


pub struct Text {
    font_system: FontSystem,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
    buffer: Buffer,
    cache: Cache,
    swash_cache: SwashCache,
}

pub struct BufferDescriptor {
    width: u32,
    height: u32,
    metrics: Metrics,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextInstance<'a> {
    attrs: Attrs<'a>,
    bounds: glyphon::TextBounds,
    position: Vec2,
    scale: f32,
    color: Color,
}

impl Text {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: TextureFormat,
        buffer_descriptor: &BufferDescriptor,
    ) -> Self {
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
        let swash_cache = SwashCache::new();
        let mut buffer = Buffer::new(&mut font_system, buffer_descriptor.metrics);
        buffer.set_size(&mut font_system, Some(buffer_descriptor.width as f32), Some(buffer_descriptor.height as f32));

        Text {
            font_system,
            cache,
            text_atlas,
            text_renderer,
            buffer,
            swash_cache,
        }
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.buffer.set_size(&mut self.font_system, Some(size.width as f32), Some(size.height as f32));
    }

    pub fn prepare(&mut self) {

    }

    pub fn draw_text(&mut self, text: &str, attrs: Attrs) {

    }

    pub fn render(&mut self) {

    }
}