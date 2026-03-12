pub mod app;
pub mod input;
pub mod renderer;
pub mod scene;

// Re-export
pub use wgpu;
pub use winit::event::WindowEvent;

/// Handles the main engine loop. Calls into the [scene::Scene] at significant points during the
/// loop.
///
/// Typical use:
/// ```ignore
/// fn main() {
///     granite::run(|renderer: &mut Renderer| Scene::new(renderer));
/// }
/// ```
#[inline]
pub fn run<Builder>(builder: Builder)
where
    Builder: app::SceneBuilder,
{
    winit::event_loop::EventLoop::new()
        .expect("could not create event loop")
        .run_app(&mut app::App::Suspended {
            builder: Some(builder),
        })
        .expect("could not run application")
}
