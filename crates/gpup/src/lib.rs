pub mod app;
pub mod input;
pub mod renderer;
pub mod scene;

pub mod prelude {
    pub use super::RunError;
    pub use super::WindowEvent;
    pub use super::app::{App, SceneBuilder};
    pub use super::input::InputState;
    pub use super::renderer::{Frame, Renderer};
    pub use super::scene::Scene;
}

// Re-export
pub use wgpu;
pub use winit::event::WindowEvent;

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
    CreateRenderer(#[from] renderer::RendererCreateError),
}

/// Handles the main engine loop. Calls into the [scene::Scene] at significant points during the
/// loop.
///
/// Typical use:
/// ```ignore
/// fn main() -> Result<(), gpup::RunError> {
///     gpup::run(|renderer: &mut Renderer| Scene::new(renderer))
/// }
/// ```
///
/// # Errors
/// Returns an error if event-loop creation or execution, window creation, or
/// renderer initialization fails.
#[inline]
pub fn run<Builder>(builder: Builder) -> Result<(), RunError>
where
    Builder: app::SceneBuilder,
{
    let event_loop = winit::event_loop::EventLoop::new()?;
    let mut app = app::App::Suspended {
        builder: Some(builder),
    };
    let result = event_loop.run_app(&mut app);

    // Preserve the startup failure even if event-loop shutdown also fails.
    match app {
        app::App::Failed { error } => Err(error),
        _ => result.map_err(RunError::from),
    }
}
