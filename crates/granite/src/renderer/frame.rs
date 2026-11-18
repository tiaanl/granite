use crate::SurfaceSize;

/// An active window frame containing the swapchain view and surface metadata.
pub struct Frame {
    pub(super) surface_texture: wgpu::SurfaceTexture,

    pub view: wgpu::TextureView,
    pub size: SurfaceSize,
    pub scale_factor: f64,
    pub format: wgpu::TextureFormat,
}

impl Frame {
    pub(super) fn new(
        surface_texture: wgpu::SurfaceTexture,
        view: wgpu::TextureView,
        size: SurfaceSize,
        scale_factor: f64,
        format: wgpu::TextureFormat,
    ) -> Self {
        Self {
            surface_texture,
            view,
            size,
            scale_factor,
            format,
        }
    }
}
