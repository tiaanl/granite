use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes, WindowId},
};

use crate::{
    input::InputState,
    renderer::{Frame, Renderer, Surface},
    scene::{Scene, SceneEvent},
};

pub trait NewScene {
    type Target: Scene;

    fn new(&self, surface: &Surface, renderer: &Renderer) -> Self::Target;
}

impl<T, F> NewScene for F
where
    T: Scene,
    F: Fn(&Surface, &Renderer) -> T,
{
    type Target = T;

    fn new(&self, surface: &Surface, renderer: &Renderer) -> Self::Target {
        self(surface, renderer)
    }
}

/// The global state of the engine. Implements the [ApplicationHandler] for winit to drive the main
/// window.
pub enum App<S, New>
where
    S: Scene,
    New: NewScene<Target = S>,
{
    /// The application is in a suspended state.
    Suspended { new: New },
    /// The application was resumed and is not actively running.
    Resumed {
        /// A handle to the main window runing our renderer.
        window: Arc<Window>,
        /// The renderer.
        renderer: Renderer,
        /// Keep track of the input state.
        input: InputState,
        /// The use [Scene] we are interacting with.
        scene: New::Target,
    },
}

impl<S, New> ApplicationHandler for App<S, New>
where
    S: Scene,
    New: NewScene<Target = S>,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let App::Suspended { new } = self else {
            panic!("App already resumed.");
        };

        let window = Arc::new(
            event_loop
                .create_window(WindowAttributes::default().with_title("wGPU Mechinical Arm"))
                .unwrap(),
        );

        let renderer = Renderer::new(Arc::clone(&window));
        let s = renderer.surface_inner.read().unwrap().surface();

        let scene = new.new(&s, &renderer);

        *self = Self::Resumed {
            window,
            renderer,
            input: InputState::default(),
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
            renderer,
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
                renderer.resize(size);

                let event = SceneEvent::WindowResized {
                    width: size.width,
                    height: size.height,
                };
                scene.event(&event);
            }

            WindowEvent::RedrawRequested => {
                scene.update(input, 1.0);

                input.reset_current_frame();

                let surface_texture = renderer.surface_inner.read().unwrap().get_current();
                let view = surface_texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let encoder =
                    renderer
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("main_command_encoder"),
                        });

                let mut frame = Frame {
                    renderer,
                    encoder,
                    view,
                };

                scene.render(
                    &renderer.surface_inner.read().unwrap().surface(),
                    &mut frame,
                );

                renderer
                    .queue
                    .submit(std::iter::once(frame.encoder.finish()));

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
