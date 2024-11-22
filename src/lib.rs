//! Handles the main engine loop. Calls into the [Scene] at significant points during the loop.

mod app;
mod engine;
mod input;

pub mod camera;
pub mod mesh;
pub mod scene;

// Re-export
pub use glam;
pub use wgpu;

pub use engine::*;
pub use input::*;
