use glam::UVec2;

/// An active window frame containing the swapchain view and command encoder.
pub struct Frame {
    surface_texture: wgpu::SurfaceTexture,

    pub encoder: wgpu::CommandEncoder,
    pub view: wgpu::TextureView,
    pub surface_size: UVec2,
    pub surface_format: wgpu::TextureFormat,
}

impl Frame {
    pub(super) fn new(
        encoder: wgpu::CommandEncoder,
        view: wgpu::TextureView,
        surface_texture: wgpu::SurfaceTexture,
        surface_size: UVec2,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        Self {
            encoder,
            view,
            surface_texture,
            surface_size,
            surface_format,
        }
    }

    pub(super) fn finish(self) -> (wgpu::CommandBuffer, wgpu::SurfaceTexture) {
        (self.encoder.finish(), self.surface_texture)
    }
}
