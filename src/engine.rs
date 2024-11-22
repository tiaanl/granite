use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};

use winit::{dpi::PhysicalSize, event_loop::EventLoop};

use crate::{
    app::{App, Surface},
    scene::Scene,
};

/// An user facing interface into the engine.
pub struct Engine {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    width: u32,
    height: u32,
    surface: Arc<Mutex<Surface>>,

    /// If this is set, the engine has to switch to this new [Scene] when possible.
    transition_scene: RefCell<Option<Box<dyn Scene>>>,
}

impl Engine {
    pub(crate) fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        width: u32,
        height: u32,
        surface: Arc<Mutex<Surface>>,
    ) -> Self {
        Self {
            device,
            queue,
            width,
            height,
            surface,
            transition_scene: RefCell::new(None),
        }
    }

    pub(crate) fn take_transition_scene(&self) -> Option<Box<dyn Scene>> {
        self.transition_scene.take()
    }

    pub(crate) fn resize(&mut self, size: PhysicalSize<u32>) {
        self.width = size.width;
        self.height = size.height;
    }
}

impl Engine {
    /// Start the engine, running the `startup` once the engine is initialized.
    pub fn start(startup: impl FnOnce(&Engine) + 'static) {
        let event_loop = EventLoop::new().expect("Could not create event loop.");
        let mut app = App::Suspended {
            startup: Some(Box::new(startup)),
        };
        event_loop
            .run_app(&mut app)
            .expect("Event loop id not run successfully.");
    }

    /// Return a reference to the render device.
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Return a reference to the render queue.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn window_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Return the format of the primary render surface.
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        let surface = self.surface.lock().unwrap();
        surface.config().format
    }

    /// Request that the engine switches to the given [Scene].
    pub fn switch_scene(&self, scene: Box<dyn Scene>) {
        let mut transition_scene = self.transition_scene.borrow_mut();
        *transition_scene = Some(scene);
    }
}
