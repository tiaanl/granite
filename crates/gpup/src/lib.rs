mod app;
mod input;
mod input_state;
mod renderer;
mod scene;
mod surface_size;
mod system_event;

pub use app::AppHandle;
pub use input::{InputEvent, KeyCode, MouseButton, MousePosition};
pub use input_state::InputState;
pub use renderer::{Frame, NextFrameError, Renderer, RendererCreateError};
pub use scene::Scene;
pub use surface_size::SurfaceSize;
pub use system_event::SystemEvent;

// Re-export
pub use wgpu;

use winit::event_loop::EventLoop;

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
    /// The scene builder failed.
    #[error("Could not build the scene. {0}")]
    BuildScene(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
}

pub struct RunContext<'a> {
    /// The renderer to use when initializing graphics resources.
    pub renderer: &'a Renderer,
    /// The application handle. Clone it to retain it in the scene.
    pub app_handle: &'a AppHandle,
}

/// Creates a window and renderer, then runs the scene's event loop until the application exits.
///
/// ```no_run
/// use gpup::{Frame, Renderer, RunContext, Scene};
///
/// struct MyScene;
///
/// impl MyScene {
///     fn new(context: RunContext<'_>) -> Self {
///         // Initialize graphics using context.renderer.
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
pub fn run<S>(build_scene: impl FnOnce(RunContext<'_>) -> S) -> Result<(), RunError>
where
    S: self::Scene,
{
    try_run(|context| Ok::<_, std::convert::Infallible>(build_scene(context)))
}

/// Creates a window and renderer, then builds and runs a scene that may fail to initialize.
///
/// Use [`run`] when scene construction is infallible.
///
/// ```no_run
/// use gpup::{Frame, Renderer, RunContext, Scene};
///
/// struct MyScene {
///     shader: String,
/// }
///
/// impl MyScene {
///     fn new(context: RunContext<'_>) -> Result<Self, std::io::Error> {
///         let shader = std::fs::read_to_string("shader.wgsl")?;
///         Ok(Self { shader })
///     }
/// }
///
/// impl Scene for MyScene {
///     fn frame(&mut self, renderer: &Renderer, frame: &Frame) {}
/// }
///
/// fn main() -> Result<(), gpup::RunError> {
///     gpup::try_run(MyScene::new)
/// }
/// ```
#[inline]
pub fn try_run<SceneType, BuildError>(
    build_scene: impl FnOnce(RunContext<'_>) -> Result<SceneType, BuildError>,
) -> Result<(), RunError>
where
    SceneType: Scene,
    BuildError: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
{
    let event_loop: EventLoop<app::AppEvent> = EventLoop::with_user_event().build()?;

    let proxy = event_loop.create_proxy();
    let app_handle = AppHandle::new(proxy);

    let mut app = app::App::Suspended {
        build_scene: Some(move |renderer: &Renderer| {
            build_scene(RunContext {
                renderer,
                app_handle: &app_handle,
            })
            .map_err(|error| RunError::BuildScene(error.into()))
        }),
    };
    let result = event_loop.run_app(&mut app);

    // Preserve the startup failure even if event-loop shutdown also fails.
    match app {
        app::App::Failed { error } => Err(error),
        _ => result.map_err(RunError::from),
    }
}
