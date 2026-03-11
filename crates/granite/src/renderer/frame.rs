/// An active window frame containing the swapchain view and surface metadata.
pub struct Frame {
    surface_texture: wgpu::SurfaceTexture,

    pub view: wgpu::TextureView,
    pub surface_size: (u32, u32), // width, height
    pub surface_format: wgpu::TextureFormat,
}

impl Frame {
    pub(super) fn new(
        view: wgpu::TextureView,
        surface_texture: wgpu::SurfaceTexture,
        surface_size: (u32, u32),
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        Self {
            view,
            surface_texture,
            surface_size,
            surface_format,
        }
    }

    pub(super) fn present(self) {
        self.surface_texture.present();
    }
}
