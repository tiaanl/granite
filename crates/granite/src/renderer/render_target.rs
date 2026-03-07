use glam::UVec2;

#[derive(Clone, Copy)]
pub enum RenderTargetFormat {
    Rgba,
    RgbaSrgb,
}

impl RenderTargetFormat {
    pub fn to_wgpu(&self) -> wgpu::TextureFormat {
        match self {
            RenderTargetFormat::Rgba => wgpu::TextureFormat::Rgba8Unorm,
            RenderTargetFormat::RgbaSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
        }
    }
}

pub struct RenderTargetRecord {
    pub size: UVec2,
    pub format: RenderTargetFormat,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

impl RenderTargetRecord {
    pub fn create(
        device: &wgpu::Device,
        name: &str,
        size: UVec2,
        format: RenderTargetFormat,
    ) -> Self {
        let extent = wgpu::Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(name),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: format.to_wgpu(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            size,
            format,
            texture,
            view,
        }
    }
}
