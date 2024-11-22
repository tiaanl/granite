use std::sync::{Arc, Mutex};

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes, WindowId},
};

use crate::{
    input::InputState,
    scene::{Scene, SceneEvent},
    Engine,
};

/// A [wgpu::Surface] and it current configuration.
pub struct Surface {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
}

impl Surface {
    fn from_window(
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        window: Arc<Window>,
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

    pub(crate) fn config(&self) -> &wgpu::SurfaceConfiguration {
        &self.config
    }
}

/// The global state of the engine. Implements the [ApplicationHandler] for winit to drive the main
/// window.
pub enum App {
    /// The application is in a suspended state.
    Suspended {
        startup: Option<Box<dyn FnOnce(&Engine)>>,
    },
    /// The application was resumed and is not actively running.
    Resumed {
        /// A handle to the main window runing our renderer.
        window: Arc<Window>,
        /// Render device.
        device: Arc<wgpu::Device>,
        /// Render queue.
        queue: Arc<wgpu::Queue>,
        /// The main surface the renderer is drawing to.
        surface: Arc<Mutex<Surface>>,
        /// Keep track of the input state.
        input: InputState,
        /// The engine interface for users.
        engine: Engine,
        /// The active scene being rendered.
        scene: Option<Box<dyn Scene>>,
    },
}

impl App {
    async fn init_renderer(window: Arc<Window>) -> (wgpu::Device, wgpu::Queue, Surface) {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase::default())
            .await
            .expect("Could not get adapter.");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .expect("Could not request device.");

        let surface = Surface::from_window(&instance, &adapter, &device, window)
            .expect("Could not create surface.");

        (device, queue, surface)
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let startup = match self {
            Self::Suspended { startup } => startup.take(),
            Self::Resumed { .. } => panic!("Why are we already resumed?"),
        };

        let window = Arc::new(
            event_loop
                .create_window(WindowAttributes::default().with_title("wGPU Mechinical Arm"))
                .unwrap(),
        );

        let (device, queue, surface) = pollster::block_on(Self::init_renderer(Arc::clone(&window)));
        let device = Arc::new(device);
        let queue = Arc::new(queue);
        let surface = Arc::new(Mutex::new(surface));

        let PhysicalSize { width, height } = window.inner_size();

        let engine = Engine::new(
            Arc::clone(&device),
            Arc::clone(&queue),
            width,
            height,
            Arc::clone(&surface),
        );

        let scene = if let Some(startup) = startup {
            // Run the engine startup function, giving it access to the [Engine].
            startup(&engine);

            // If the startup code set a new [Scene], we set it.
            engine.take_transition_scene()
        } else {
            None
        };

        *self = Self::Resumed {
            window,
            device,
            queue,
            surface,
            input: InputState::default(),
            engine,
            scene,
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Self::Resumed {
            window,
            device,
            queue,
            surface,
            engine,
            input,
            scene,
            ..
        } = self
        else {
            // Window events while we are suspended?
            return;
        };

        if window_id != window.id() {
            // Not our window?
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                {
                    let mut surface = surface.lock().expect("No access to surface!");
                    surface.resize(device.as_ref(), size);
                }
                engine.resize(size);

                if let Some(scene) = scene {
                    scene.engine_event(
                        &engine,
                        &SceneEvent::WindowResized {
                            width: size.width,
                            height: size.height,
                        },
                    );
                }
            }

            WindowEvent::RedrawRequested => {
                if let Some(scene) = scene {
                    scene.input(input);
                    scene.update(1.0);
                    scene.render_update(queue.as_ref());
                }

                input.reset_current_frame();

                let surface_texture = surface
                    .lock()
                    .unwrap()
                    .surface
                    .get_current_texture()
                    .expect("Could not get current surface.");
                let view = surface_texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("main_command_encoder"),
                });

                if let Some(scene) = scene {
                    scene.render(&mut encoder, &view);
                }

                queue.submit(std::iter::once(encoder.finish()));

                surface_texture.present();

                window.request_redraw();
            }

            event => {
                // Consume the event.
                input.handle_window_event(event);
            }
        }
    }
}
