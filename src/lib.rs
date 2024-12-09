//! Handles the main engine loop. Calls into the [Scene] at significant points during the loop.

mod app;
mod input;
mod renderer;

mod camera;
mod mesh;
mod scene;

// Re-export
pub use glam;
pub use wgpu;

pub mod prelude {
    pub use super::app::NewScene;
    pub use super::camera::*;
    pub use super::input::*;
    pub use super::renderer::{Frame, Renderer, Surface};
    pub use super::scene::*;
}

#[inline]
pub fn run<S, New>(new: New)
where
    S: scene::Scene,
    New: app::NewScene<Target = S>,
{
    winit::event_loop::EventLoop::new()
        .expect("could not create event loop")
        .run_app(&mut app::App::Suspended { new })
        .unwrap();
}
