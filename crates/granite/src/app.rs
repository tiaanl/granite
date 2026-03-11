use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes, WindowId},
};

use crate::{renderer::Renderer, scene::Scene};

pub trait SceneBuilder {
    type Target: Scene;

    fn build(self, renderer: &mut Renderer) -> Self::Target;
}

impl<T, F> SceneBuilder for F
where
    T: Scene,
    F: for<'a> FnOnce(&'a mut Renderer) -> T,
{
    type Target = T;

    fn build(self, renderer: &mut Renderer) -> Self::Target {
        self(renderer)
    }
}

/// The global state of the engine. Implements the [ApplicationHandler] for [winit] to drive the
/// main window.
// Allow the clippy large_enum_variant, becuase we only have one instance of [App] and we use the
// enum as a "initialized" flag only.
#[allow(clippy::large_enum_variant)]
pub enum App<Builder>
where
    Builder: SceneBuilder,
{
    /// The application is in a suspended state.
    Suspended { builder: Option<Builder> },
    /// The application was resumed and is now actively running.
    Resumed {
        /// A handle to the main window running our renderer.
        window: Arc<Window>,
        /// The renderer.
        renderer: Renderer,
        /// The [Scene] we are interacting with.
        scene: Builder::Target,
        /// The last [std::time::Instant] that a frame was rendered to the display.
        last_frame_time: std::time::Instant,
    },
}

impl<Builder> ApplicationHandler for App<Builder>
where
    Builder: SceneBuilder,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let App::Suspended { builder } = self else {
            panic!("App already resumed.");
        };

        let builder = builder.take().expect("App already resumed.");

        let window = Arc::new(
            event_loop
                .create_window(WindowAttributes::default())
                .unwrap(),
        );

        let PhysicalSize { width, height } = window.inner_size();

        let mut renderer =
            Renderer::new(Arc::clone(&window), width, height).expect("Could not create renderer");
        let scene = builder.build(&mut renderer);

        *self = Self::Resumed {
            window,
            renderer,
            scene,
            last_frame_time: std::time::Instant::now(),
        };
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
            scene,
            last_frame_time,
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

        scene.window_event(&event);

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Resized(PhysicalSize { width, height }) => {
                renderer.resize(width, height);
            }

            WindowEvent::RedrawRequested => {
                // The amount of seconds elapsed since the last frame was presented.
                let now = std::time::Instant::now();
                let delta_time = (now - *last_frame_time).as_secs_f32();
                *last_frame_time = now;

                {
                    let frame = renderer.begin_frame().expect("Could not begin frame");
                    scene.frame(renderer, &frame, delta_time);
                    renderer.submit_frame(frame);
                }

                window.request_redraw();
            }

            WindowEvent::Occluded(_) => {}

            _ => {}
        }
    }
}
