use std::sync::Arc;

use winit::{dpi::PhysicalSize, window::Window};

pub struct Renderer {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub(crate) surface_inner: SurfaceInner,
}

impl Renderer {
    pub fn new(window: Arc<Window>) -> Self {
        use pollster::block_on;

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

        let adapter =
            block_on(instance.request_adapter(&wgpu::RequestAdapterOptionsBase::default()))
                .expect("Could not get adapter.");

        let (device, queue) =
            block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
                .expect("Could not request device.");

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let surface_inner = SurfaceInner::new(window, &instance, &adapter, &device)
            .expect("Could not create surface.");

        Self {
            device,
            queue,
            surface_inner,
        }
    }

    pub(crate) fn resize(&mut self, size: PhysicalSize<u32>) {
        self.surface_inner.resize(&self.device, size);
    }
}

/// Details of a [Surface].
pub struct SurfaceConfig {
    pub format: wgpu::TextureFormat,
    pub width: u32,
    pub height: u32,
}

impl From<&wgpu::SurfaceConfiguration> for SurfaceConfig {
    fn from(value: &wgpu::SurfaceConfiguration) -> Self {
        Self {
            format: value.format,
            width: value.width,
            height: value.height,
        }
    }
}

/// A thin object passed to describe the window's surface.
pub struct Surface {
    texture: wgpu::SurfaceTexture,
    pub view: wgpu::TextureView,
    pub config: SurfaceConfig,
}

impl Surface {
    /// Consume the [Surface] and present it to the screen.
    pub(crate) fn present(self) {
        self.texture.present();
    }
}

/// A [wgpu::Surface] and it current configuration.
pub(crate) struct SurfaceInner {
    surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
}

impl SurfaceInner {
    fn new(
        window: Arc<Window>,
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
    ) -> Result<Self, wgpu::CreateSurfaceError> {
        let PhysicalSize { width, height } = window.inner_size();

        let surface = instance.create_surface(window)?;
        let config = surface
            .get_default_config(adapter, width, height)
            .expect("Could not get surface configuration.");

        surface.configure(device, &config);

        Ok(Self { surface, config })
    }

    fn resize(&mut self, device: &wgpu::Device, size: PhysicalSize<u32>) {
        self.config.width = size.width.max(1);
        self.config.height = size.height.max(1);
        self.surface.configure(device, &self.config);
    }

    pub(crate) fn get_current_surface(&self) -> Surface {
        let texture = self
            .surface
            .get_current_texture()
            .expect("Could not get current surface.");

        let view = texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        Surface {
            texture,
            view,
            config: SurfaceConfig {
                format: self.config.format,
                width: self.config.width,
                height: self.config.height,
            },
        }
    }
}
