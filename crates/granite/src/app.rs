use std::sync::Arc;

use glam::UVec2;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes, WindowId},
};

use crate::{
    input::InputState,
    renderer::Renderer,
    scene::{Scene, SceneEvent},
};

pub trait SceneBuilder {
    type Target: Scene;

    fn build(&self, renderer: &mut Renderer) -> Self::Target;
}

impl<T, F> SceneBuilder for F
where
    T: Scene,
    F: Fn(&mut Renderer) -> T,
{
    type Target = T;

    fn build(&self, renderer: &mut Renderer) -> Self::Target {
        self(renderer)
    }
}

/// The global state of the engine. Implements the [ApplicationHandler] for [winit] to drive the
/// main window.
// Allow the clippy large_enum_variant, becuase we only have one instance of [App] and we use the
// enum as a "initialized" flag only.
#[allow(clippy::large_enum_variant)]
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
        /// The last [std::time::Instant] that a frame was rendered to the display.
        last_frame_time: std::time::Instant,
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

        let PhysicalSize { width, height } = window.inner_size();

        let mut renderer = Renderer::new(Arc::clone(&window), UVec2::new(width, height))
            .expect("Could not create renderer");
        let scene = builder.build(&mut renderer);

        *self = Self::Resumed {
            window,
            renderer,
            input: InputState::default(),
            scene,
            last_frame_time: std::time::Instant::now(),
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

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Resized(PhysicalSize { width, height }) => {
                renderer.resize(UVec2::new(width, height));

                scene.event(SceneEvent::WindowResized { width, height });
            }

            WindowEvent::RedrawRequested => {
                // The amount of seconds elapsed since the last frame was presented.
                let now = std::time::Instant::now();
                let delta_time = (now - *last_frame_time).as_secs_f32();
                *last_frame_time = now;

                scene.update(input, delta_time);

                input.reset_current_frame();

                {
                    let mut frame = renderer.begin_frame().expect("Could not begin frame");

                    scene.render(&mut frame);

                    renderer
                        .submit_frame(frame)
                        .expect("Could not submit frame");
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
