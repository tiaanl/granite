use glam::UVec2;

/// Pixel format for a texture resource.
pub enum TextureFormat {
    /// 8-bit RGBA, linear color space.
    Rgba,
    /// 8-bit RGBA, sRGB color space.
    RgbaSrgb,
    /// Single-channel 8-bit (red only).
    Mono,
}

impl TextureFormat {
    /// Returns the number of bytes per pixel for this format.
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
