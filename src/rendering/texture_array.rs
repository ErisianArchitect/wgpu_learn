use std::path::Path;

use image::GenericImageView;

// fn log2_u32(n: u32) -> u32 {
//     debug_assert!(n > 0);
//     31 - n.leading_zeros()
// }

// #[test]
// fn log2_test() {
//     assert_eq!(log2_u32(1), 0);
//     assert_eq!(log2_u32(32), 5);
// }

#[derive(Debug, thiserror::Error)]
pub enum TexArrErr {
    #[error("No paths provided.")]
    NoPaths,
    #[error("Failed to load image: {0}")]
    FailedToLoadImage(#[from] image::ImageError),
    #[error("Image {index} has dimensions of {dimensions:?}, expected {expected:?}.")]
    MismatchedDimensions {
        index: u32,
        dimensions: (u32, u32),
        expected: (u32, u32),
    }
}

pub struct TextureArray {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub samplers: TextureArraySamplers,
    pub format: wgpu::TextureFormat,
    pub dimensions: (u32, u32),
    pub layer_count: u32,
}

impl TextureArray {
    pub fn from_files<P: AsRef<Path>>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        paths: &[P],
        label: Option<&str>,
        format: wgpu::TextureFormat,
    ) -> Result<Self, TexArrErr> {
        if paths.is_empty() {
            return Err(TexArrErr::NoPaths);
        }

        let first_img = image::open(paths[0].as_ref())?;
        let (width, height) = first_img.dimensions();

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: paths.len() as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let bytes_per_row = Some(4 * width);
        let rows_per_image = Some(height);

        for (i, path) in paths.iter().enumerate() {
            let img = image::open(path.as_ref())?;

            // Ensure all images have the same dimensions
            let (img_width, img_height) = img.dimensions();

            if (img_width, img_height) != (width, height) {
                return Err(TexArrErr::MismatchedDimensions {
                    index: i as u32,
                    dimensions: (img_width, img_height),
                    expected: (width, height),
                });
            }

            let rgba = img.to_rgba8();

            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: i as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &rgba,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row,
                    rows_per_image,
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label,
            format: Some(format),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        Ok(Self {
            texture,
            view,
            format,
            samplers: Self::create_samplers(device),
            dimensions: (width, height),
            layer_count: paths.len() as u32,
        })
    }

    pub fn bind_group(&self, device: &wgpu::Device) -> TextureArrayBindGroup {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Texture Array Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                // Near Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // Far Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Array Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.view),
                },
                // Near Sampler
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.near),
                },
                // Far Sampler
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.samplers.far),
                }
            ],
        });
        TextureArrayBindGroup {
            bind_group,
            bind_group_layout,
        }
    }

    pub fn create_samplers(device: &wgpu::Device) -> TextureArraySamplers {
        let far = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Texture Array Far Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            anisotropy_clamp: 16,
            ..Default::default()
        });
        let near = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Texture Array Near Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        TextureArraySamplers { near, far }
    }

    pub fn texel_to_uv(&self, texpos: glam::Vec2) -> glam::Vec2 {
        glam::vec2(
            texpos.x / self.dimensions.0 as f32,
            texpos.y / self.dimensions.1 as f32
        )
    }
}

pub struct TextureArraySamplers {
    near: wgpu::Sampler,
    far: wgpu::Sampler,
}

pub struct TextureArrayBindGroup {
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}