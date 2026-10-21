use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow},
    window::{Window, WindowAttributes, WindowId},
};

use crate::{InputEvent, RunError, SystemEvent, renderer::Renderer, scene::Scene};

pub trait SceneBuilder {
    type Scene: Scene;

    fn build(self, renderer: &mut Renderer) -> Self::Scene;
}

impl<T, F> SceneBuilder for F
where
    T: Scene,
    F: for<'a> FnOnce(&'a mut Renderer) -> T,
{
    type Scene = T;

    fn build(self, renderer: &mut Renderer) -> Self::Scene {
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
    /// Startup failed; the error is retained until the event loop exits.
    Failed { error: RunError },
    /// The application was resumed and is now actively running.
    Resumed {
        /// A handle to the main window running our renderer.
        window: Arc<Window>,
        /// Whether the window is completely hidden from view.
        occluded: bool,
        /// The renderer.
        renderer: Renderer,
        /// The [Scene] we are interacting with.
        scene: Builder::Scene,
        /// The last update time, or `None` when the frame clock is paused.
        last_frame_time: Option<std::time::Instant>,
    },
}

impl<Builder> App<Builder>
where
    Builder: SceneBuilder,
{
    fn can_render(&self) -> bool {
        let Self::Resumed {
            window, occluded, ..
        } = self
        else {
            return false;
        };

        let size = window.inner_size();
        !occluded && size.width > 0 && size.height > 0 && window.is_minimized() != Some(true)
    }
}

impl<Builder> ApplicationHandler for App<Builder>
where
    Builder: SceneBuilder,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let App::Suspended { builder } = self else {
            // Winit may emit redundant resume callbacks.
            return;
        };

        let Some(builder) = builder.take() else {
            return;
        };

        let window = match event_loop.create_window(WindowAttributes::default()) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                *self = Self::Failed {
                    error: RunError::CreateWindow(error),
                };
                event_loop.exit();
                return;
            }
        };

        let mut renderer = match Renderer::new(Arc::clone(&window)) {
            Ok(renderer) => renderer,
            Err(error) => {
                *self = Self::Failed {
                    error: RunError::CreateRenderer(error),
                };
                event_loop.exit();
                return;
            }
        };

        // Build the scene with the initialized renderer.
        let scene = builder.build(&mut renderer);

        *self = Self::Resumed {
            window,
            occluded: false,
            renderer,
            scene,
            last_frame_time: Some(std::time::Instant::now()),
        };
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if !self.can_render() {
            if let Self::Resumed {
                last_frame_time, ..
            } = self
            {
                *last_frame_time = None;
            }
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        }

        event_loop.set_control_flow(ControlFlow::Poll);
        if let Self::Resumed { window, .. } = self {
            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let can_render = self.can_render();

        let Self::Resumed {
            window,
            occluded,
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

        // A redraw may already be queued when the window becomes hidden.
        if matches!(event, WindowEvent::RedrawRequested) && !can_render {
            *last_frame_time = None;
            return;
        }

        if let Some(input_event) = InputEvent::from_window_event(&event) {
            scene.input(&input_event);
        }

        if let Some(system_event) = SystemEvent::from_window_event(&event) {
            scene.system_event(&system_event);
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Occluded(value) => {
                *occluded = value;
                if value {
                    *last_frame_time = None;
                }
            }

            WindowEvent::Resized(PhysicalSize { width, height }) => {
                renderer.resize(width, height);
                if width == 0 || height == 0 {
                    *last_frame_time = None;
                }
            }

            WindowEvent::RedrawRequested => {
                let delta_time = {
                    let now = std::time::Instant::now();
                    last_frame_time
                        .replace(now)
                        .map_or(0.0, |previous| now.duration_since(previous).as_secs_f32())
                };

                scene.update(delta_time);

                match renderer.next_frame() {
                    Ok(Some(frame)) => {
                        scene.frame(renderer, &frame);
                        renderer.present(frame);
                    }
                    Ok(None) => {
                        // Skipping frame.
                    }
                    Err(err) => {
                        tracing::warn!("Could not acquire next frame. {err}");
                    }
                };
            }

            _ => {}
        }
    }
}
