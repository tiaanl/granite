use std::collections::HashSet;

use winit::{
    event::{ElementState, MouseScrollDelta, WindowEvent},
    keyboard::PhysicalKey,
};

pub use winit::event::MouseButton;
pub use winit::keyboard::KeyCode;

#[derive(Clone, Copy, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct MousePosition {
    pub x: i32,
    pub y: i32,
}

impl MousePosition {
    pub fn from_xy(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

impl std::ops::Sub for MousePosition {
    type Output = MousePosition;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self::Output {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

#[derive(Clone, Default)]
pub struct InputState {
    key_pressed: HashSet<KeyCode>,
    key_just_pressed: HashSet<KeyCode>,
    mouse_pressed: HashSet<MouseButton>,
    mouse_just_pressed: HashSet<MouseButton>,
    mouse_just_released: HashSet<MouseButton>,
    last_mouse_position: Option<MousePosition>,
    mouse_delta: Option<MousePosition>,
    mouse_wheel_delta: f32,
}

impl InputState {
    pub fn handle_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(key) = event.physical_key
                    && !event.repeat
                {
                    if event.state == ElementState::Pressed {
                        self.key_pressed.insert(key);
                        self.key_just_pressed.insert(key);
                    } else {
                        self.key_pressed.remove(&key);
                    }
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if state.is_pressed() {
                    self.mouse_pressed.insert(*button);
                    self.mouse_just_pressed.insert(*button);
                } else {
                    self.mouse_pressed.remove(button);
                    self.mouse_just_released.insert(*button);
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let current =
                    MousePosition::from_xy(position.x.round() as i32, position.y.round() as i32);
                self.mouse_delta = self.last_mouse_position.map(|pos| current - pos);
                self.last_mouse_position = Some(current);
            }

            WindowEvent::MouseWheel { delta, .. } => {
                self.mouse_wheel_delta = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(physical_position) => physical_position.y as f32,
                }
            }

            _ => {}
        }
    }

    /// Reset data being tracked per frame.
    pub fn reset_current_frame(&mut self) {
        self.key_just_pressed.clear();
        self.mouse_just_pressed.clear();
        self.mouse_just_released.clear();
        self.mouse_delta = None;
        self.mouse_wheel_delta = 0.0;
    }
}

impl InputState {
    #[inline]
    pub fn key_pressed(&self, key: KeyCode) -> bool {
        self.key_pressed.contains(&key)
    }

    #[inline]
    pub fn key_just_pressed(&self, key: KeyCode) -> bool {
        self.key_just_pressed.contains(&key)
    }

    #[inline]
    pub fn mouse_position(&self) -> Option<MousePosition> {
        self.last_mouse_position
    }

    #[inline]
    pub fn mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse_pressed.contains(&button)
    }

    #[inline]
    pub fn mouse_just_pressed(&self, button: MouseButton) -> bool {
        self.mouse_just_pressed.contains(&button)
    }

    #[inline]
    pub fn mouse_just_released(&self, button: MouseButton) -> bool {
        self.mouse_just_released.contains(&button)
    }

    #[inline]
    pub fn mouse_delta(&self) -> Option<MousePosition> {
        self.mouse_delta
    }

    #[inline]
    pub fn mouse_wheel_delta(&self) -> f32 {
        self.mouse_wheel_delta
    }
}
