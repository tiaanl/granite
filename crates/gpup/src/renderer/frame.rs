use crate::SurfaceSize;

/// An active window frame containing the swapchain view and surface metadata.
pub struct Frame {
    pub(super) surface_texture: wgpu::SurfaceTexture,

    pub view: wgpu::TextureView,
    pub size: SurfaceSize,
    pub format: wgpu::TextureFormat,
}

impl Frame {
    pub(super) fn new(
        surface_texture: wgpu::SurfaceTexture,
        view: wgpu::TextureView,
        size: SurfaceSize,
        format: wgpu::TextureFormat,
    ) -> Self {
        Self {
            surface_texture,
            view,
            size,
            format,
        }
    }
}
