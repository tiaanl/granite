use glam::UVec2;

pub enum TextureFormat {
    Rgba,
    RgbaSrgb,
    Mono,
}

impl TextureFormat {
    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            TextureFormat::Rgba | TextureFormat::RgbaSrgb => 4,
            TextureFormat::Mono => 1,
        }
    }

    pub(crate) fn to_wgpu(&self) -> wgpu::TextureFormat {
        match self {
            TextureFormat::Rgba => wgpu::TextureFormat::Rgba8Unorm,
            TextureFormat::RgbaSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
            TextureFormat::Mono => wgpu::TextureFormat::R8Unorm,
        }
    }
}

pub struct TextureRecord {
    pub size: UVec2,
    pub format: TextureFormat,
    pub _texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}
