use std::sync::Arc;

use parking_lot::Mutex;
use winit::{dpi::PhysicalSize, window::Window};

pub struct Renderer {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub(crate) surface_inner: Arc<Mutex<SurfaceInner>>,
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

        let surface = Arc::new(Mutex::new(
            SurfaceInner::new(window, &instance, &adapter, &device)
                .expect("Could not create surface."),
        ));

        Self {
            device,
            queue,
            surface_inner: surface,
        }
    }

    pub(crate) fn resize(&mut self, size: PhysicalSize<u32>) {
        let mut surface = self.surface_inner.lock();
        surface.resize(self.device.as_ref(), size);
    }
}

/// A thin object passed to the [Scene::render] function.
pub struct Surface {
    pub format: wgpu::TextureFormat,
    pub width: u32,
    pub height: u32,
}

/// A [wgpu::Surface] and it current configuration.
pub(crate) struct SurfaceInner {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
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

    pub(crate) fn get_current(&self) -> wgpu::SurfaceTexture {
        self.surface.get_current_texture().unwrap()
    }

    pub(crate) fn surface(&self) -> Surface {
        Surface {
            format: self.config.format,
            width: self.config.width,
            height: self.config.height,
        }
    }
}

/// Holds some data about the current frame about to be rendered to by the [Scene].
pub struct Frame<'r> {
    pub renderer: &'r Renderer,
    pub encoder: wgpu::CommandEncoder,
    pub view: wgpu::TextureView,
}
