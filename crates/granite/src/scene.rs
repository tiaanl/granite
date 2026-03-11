use crate::{
    WindowEvent,
    renderer::{Frame, Renderer},
};

pub trait Scene {
    fn window_event(&mut self, event: &WindowEvent) {
        let _ = event;
    }

    fn frame(&mut self, renderer: &Renderer, frame: &Frame, delta_time: f32);
}
