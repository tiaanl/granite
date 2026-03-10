use granite::{glam::UVec2, wgpu};

/// Pixel format for a render target.
#[derive(Clone, Copy)]
pub enum RenderTargetFormat {
    /// 8-bit RGBA, linear color space.
    Rgba,
    /// 8-bit RGBA, sRGB color space.
    RgbaSrgb,
}

impl RenderTargetFormat {
    pub(super) fn to_wgpu(self) -> wgpu::TextureFormat {
        match self {
            RenderTargetFormat::Rgba => wgpu::TextureFormat::Rgba8Unorm,
            RenderTargetFormat::RgbaSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
        }
    }
}

/// Specifies how a render target's dimensions are determined.
#[derive(Clone, Copy, Debug)]
pub enum RenderTargetSize {
    /// Matches the render surface size and resizes automatically when the surface is resized.
    /// Cannot be manually resized via `resize_render_target`.
    SurfaceSize,
    /// A fixed custom size, managed manually via `resize_render_target`.
    Custom(UVec2),
}

pub struct RenderTargetRecord {
    pub name: String,
    /// The size of the currently allocated GPU texture, or `UVec2::ZERO` if not yet allocated.
    pub size: UVec2,
    pub size_mode: RenderTargetSize,
    pub format: RenderTargetFormat,
    pub _texture: Option<wgpu::Texture>,
    pub view: Option<wgpu::TextureView>,
}

impl RenderTargetRecord {
    /// Creates a record for a surface-sized render target. No GPU resources are allocated yet;
    /// they are created on first use via [`RenderTargetRecord::allocate`].
    pub fn create_surface_sized(name: &str, format: RenderTargetFormat) -> Self {
        Self {
            name: name.to_string(),
            size: UVec2::ZERO,
            size_mode: RenderTargetSize::SurfaceSize,
            format,
            _texture: None,
            view: None,
        }
    }

    /// Creates a record for a custom-sized render target. No GPU resources are allocated yet;
    /// they are created on first use via [`RenderTargetRecord::allocate`].
    pub fn create_custom(name: &str, size: UVec2, format: RenderTargetFormat) -> Self {
        Self {
            name: name.to_string(),
            size,
            size_mode: RenderTargetSize::Custom(size),
            format,
            _texture: None,
            view: None,
        }
    }

    /// Allocates (or reallocates) the GPU texture at the given size.
    /// Drops any previously held texture before creating the new one.
    pub fn allocate(&mut self, device: &wgpu::Device, size: UVec2) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&self.name),
            size: wgpu::Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format.to_wgpu(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        self.view = Some(texture.create_view(&wgpu::TextureViewDescriptor::default()));
        self._texture = Some(texture);
        self.size = size;
    }
}
