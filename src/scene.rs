use crate::{
    input::InputState,
    renderer::{Frame, Surface},
};

pub enum SceneEvent {
    WindowResized { width: u32, height: u32 },
}

#[allow(unused)]
pub trait Scene {
    fn event(&mut self, event: &SceneEvent) {}
    fn update(&mut self, input: &InputState, time_delta: f32) {}
    fn render(&mut self, surface: &Surface, view: &mut Frame);
}
