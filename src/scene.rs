use crate::{input::InputState, Engine};

pub enum SceneEvent {
    WindowResized { width: u32, height: u32 },
}

pub trait Scene {
    fn engine_event(&mut self, engine: &Engine, event: &SceneEvent);

    #[allow(unused)]
    fn input(&mut self, input: &InputState) {}

    fn update(&mut self, time_delta: f32);

    fn render_update(&self, queue: &wgpu::Queue);

    fn render(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView);
}
