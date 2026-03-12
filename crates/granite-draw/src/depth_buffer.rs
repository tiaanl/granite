use glam::UVec2;

/// Specifies how a depth buffer's dimensions are determined.
#[derive(Clone, Copy, Debug)]
pub enum DepthBufferSize {
    /// Matches the render surface size and resizes automatically when the surface is resized.
    /// Cannot be manually resized via `resize_depth_buffer`.
    SurfaceSize,
    /// A fixed custom size, managed manually via `resize_depth_buffer`.
    Custom(UVec2),
}

pub struct DepthBufferRecord {
    pub name: String,
    /// The size of the currently allocated GPU texture, or `UVec2::ZERO` if not yet allocated.
    pub size: UVec2,
    pub size_mode: DepthBufferSize,
    /// Whether the currently allocated texture contents are safe to load from.
    pub initialized: bool,
    pub _texture: Option<wgpu::Texture>,
    pub view: Option<wgpu::TextureView>,
}

impl DepthBufferRecord {
    pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    /// Creates a record for a surface-sized depth buffer. No GPU resources are allocated yet;
    /// they are created on first use via [`DepthBufferRecord::allocate`].
    pub fn create_surface_sized(name: &str) -> Self {
        Self {
            name: name.to_string(),
            size: UVec2::ZERO,
            size_mode: DepthBufferSize::SurfaceSize,
            initialized: false,
            _texture: None,
            view: None,
        }
    }

    /// Creates a record for a custom-sized depth buffer. No GPU resources are allocated yet;
    /// they are created on first use via [`DepthBufferRecord::allocate`].
    pub fn create_custom(name: &str, size: UVec2) -> Self {
        Self {
            name: name.to_string(),
            size,
            size_mode: DepthBufferSize::Custom(size),
            initialized: false,
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
            format: Self::FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        self.view = Some(texture.create_view(&wgpu::TextureViewDescriptor::default()));
        self._texture = Some(texture);
        self.size = size;
        self.initialized = false;
    }
}
