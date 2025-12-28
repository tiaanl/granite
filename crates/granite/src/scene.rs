use crate::{
    input::InputState,
    prelude::{RenderContext, Surface},
};

pub enum SceneEvent {
    WindowResized { width: u32, height: u32 },
}

pub trait Scene {
    #[allow(unused_variables)]
    fn event(&mut self, event: &SceneEvent) {}

    #[allow(unused_variables)]
    fn update(&mut self, input: &InputState, time_delta: f32) {}

    #[must_use]
    fn render(
        &mut self,
        renderer: &RenderContext,
        surface: &Surface,
    ) -> impl Iterator<Item = wgpu::CommandBuffer>;
}
