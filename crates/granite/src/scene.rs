use crate::{input::InputState, renderer::Frame};

pub enum SceneEvent {
    WindowResized { width: u32, height: u32 },
}

pub trait Scene {
    fn event(&mut self, event: SceneEvent) {
        let _ = event;
    }

    fn update(&mut self, input: &InputState, delta_time: f32) {
        let _ = input;
        let _ = delta_time;
    }

    fn render(&mut self, frame: &mut Frame);
}
