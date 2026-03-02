mod app;
mod common;
mod input;
mod renderer;
mod scene;

// Re-export
pub use glam;
pub use wgpu;

pub mod prelude {
    pub use super::app::*;
    pub use super::input::*;
    pub use super::renderer::*;
    pub use super::scene::*;
}

/// Handles the main engine loop. Calls into the [scene::Scene] at significant points during the loop.
#[inline]
pub fn run<Scene, Builder>(builder: Builder) -> Result<(), winit::error::EventLoopError>
where
    Scene: scene::Scene,
    Builder: app::SceneBuilder<Target = Scene>,
{
    winit::event_loop::EventLoop::new()
        .expect("could not create event loop")
        .run_app(&mut app::App::Suspended { builder })
}
