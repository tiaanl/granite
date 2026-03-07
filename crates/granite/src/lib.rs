pub mod app;
pub mod common;
pub mod input;
pub mod renderer;
pub mod scene;

// Re-export
pub use encase;
pub use glam;

pub mod prelude {
    pub use super::app::*;
    pub use super::encase::ShaderType;
    pub use super::input::*;
    pub use super::renderer::*;
    pub use super::scene::*;
}

pub mod macros {
    pub use granite_macros::{AsInstanceLayout, AsUniformBuffer, AsVertexLayout};
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
