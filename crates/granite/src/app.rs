use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes, WindowId},
};

use crate::{
    input::InputState,
    prelude::SurfaceConfig,
    renderer::Renderer,
    scene::{Scene, SceneEvent},
};

pub trait SceneBuilder {
    type Target: Scene;

    fn build(&self, renderer: &Renderer, surface_config: &SurfaceConfig) -> Self::Target;
}

impl<T, F> SceneBuilder for F
where
    T: Scene,
    F: Fn(&Renderer, &SurfaceConfig) -> T,
{
    type Target = T;

    fn build(&self, renderer: &Renderer, surface: &SurfaceConfig) -> Self::Target {
        self(renderer, surface)
    }
}

/// The global state of the engine. Implements the [ApplicationHandler] for [winit] to drive the
/// main window.
pub enum App<S, Builder>
where
    S: Scene,
    Builder: SceneBuilder<Target = S>,
{
    /// The application is in a suspended state.
    Suspended { builder: Builder },
    /// The application was resumed and is not actively running.
    Resumed {
        /// A handle to the main window runing our renderer.
        window: Arc<Window>,
        /// The renderer.
        renderer: Renderer,
        /// Keep track of the input state.
        input: InputState,
        /// The use [Scene] we are interacting with.
        scene: Builder::Target,
    },
}

impl<S, Builder> ApplicationHandler for App<S, Builder>
where
    S: Scene,
    Builder: SceneBuilder<Target = S>,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let App::Suspended { builder } = self else {
            panic!("App already resumed.");
        };

        let window = Arc::new(
            event_loop
                .create_window(WindowAttributes::default())
                .unwrap(),
        );

        let renderer = Renderer::new(Arc::clone(&window));
        let surface_config = SurfaceConfig::from(&renderer.surface_inner.config);
        let scene = builder.build(&renderer, &surface_config);

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

                {
                    let surface = renderer.surface_inner.get_current_surface();
                    renderer.queue.submit(scene.render(renderer, &surface));
                    surface.present();
                }

                window.request_redraw();
            }

            event => {
                // Consume the event.
                input.handle_window_event(event);
            }
        }
    }
}
