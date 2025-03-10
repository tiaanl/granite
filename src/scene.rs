use crate::{
    input::InputState,
    prelude::{Renderer, Surface, SurfaceConfig},
};

pub enum SceneEvent {
    WindowResized { width: u32, height: u32 },
}

pub trait Scene {
    fn event(&mut self, event: &SceneEvent) {}
    fn update(&mut self, input: &InputState, time_delta: f32) {}
    fn render(
        &mut self,
        renderer: &Renderer,
        surface: &Surface,
    ) -> impl Iterator<Item = wgpu::CommandBuffer>;
}
