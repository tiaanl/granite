use crate::{
    InputEvent, SystemEvent,
    renderer::{Frame, Renderer},
};

pub trait Scene {
    fn system_event(&mut self, event: &SystemEvent) {
        let _ = event;
    }

    fn input(&mut self, event: &InputEvent) {
        let _ = event;
    }

    fn update(&mut self, delta_time: f32) {
        let _ = delta_time;
    }

    fn frame(&mut self, renderer: &Renderer, frame: &Frame);
}
