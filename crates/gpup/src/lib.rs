mod app;
mod input;
mod input_state;
mod renderer;
mod scene;
mod surface_size;
mod system_event;

pub use input::{InputEvent, KeyCode, MouseButton, MousePosition};
pub use input_state::InputState;
pub use renderer::{Frame, NextFrameError, Renderer, RendererCreateError};
pub use scene::Scene;
pub use surface_size::SurfaceSize;
pub use system_event::SystemEvent;

// Re-export
pub use wgpu;

/// Errors returned when starting or running an application.
#[derive(Debug, thiserror::Error)]
pub enum RunError {
    /// The event loop could not be created or run.
    #[error("Could not create or run the event loop. {0}")]
    EventLoop(#[from] winit::error::EventLoopError),
    /// The application window could not be created.
    #[error("Could not create the window. {0}")]
    CreateWindow(#[from] winit::error::OsError),
    /// The renderer could not be initialized.
    #[error("Could not initialize the renderer. {0}")]
    CreateRenderer(#[from] RendererCreateError),
}

/// Creates a window and renderer, then runs the scene's event loop until the application exits.
///
/// ```no_run
/// use gpup::{Frame, Renderer, Scene};
///
/// struct MyScene;
///
/// impl MyScene {
///     fn new(renderer: &mut Renderer) -> Self {
///         // Initialize graphics using the renderer.
///         Self
///     }
/// }
///
/// impl Scene for MyScene {
///     fn frame(&mut self, renderer: &Renderer, frame: &Frame) {}
/// }
///
/// fn main() -> Result<(), gpup::RunError> {
///     gpup::run(MyScene::new)
/// }
/// ```
///
/// # Errors
/// Returns an error if event-loop creation or execution, window creation, or
/// renderer initialization fails.
#[inline]
pub fn run<S>(build_scene: impl FnOnce(&Renderer) -> S) -> Result<(), RunError>
where
    S: self::Scene,
{
    let event_loop = winit::event_loop::EventLoop::new()?;
    let mut app = app::App::Suspended {
        build_scene: Some(build_scene),
    };
    let result = event_loop.run_app(&mut app);

    // Preserve the startup failure even if event-loop shutdown also fails.
    match app {
        app::App::Failed { error } => Err(error),
        _ => result.map_err(RunError::from),
    }
}
