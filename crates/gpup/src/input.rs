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

// Conversion for consistent wheel and touchpad scrolling.
const SCROLL_PIXELS_PER_LINE: f64 = 40.0;

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
            WindowEvent::Focused(false) => {
                // Cancel input without generating release actions on focus loss.
                *self = Self::default();
            }

            WindowEvent::CursorLeft { .. } => {
                // Keep held inputs intact so dragging can continue outside the window.
                self.last_mouse_position = None;
                self.mouse_delta = None;
            }

            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => {
                let PhysicalKey::Code(key) = event.physical_key else {
                    return;
                };
                if event.state == ElementState::Pressed {
                    let newly_pressed = self.key_pressed.insert(key);
                    if newly_pressed && !event.repeat && !is_synthetic {
                        self.key_just_pressed.insert(key);
                    }
                } else {
                    self.key_pressed.remove(&key);
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
                if let Some(previous) = self.last_mouse_position {
                    let movement = current - previous;
                    let accumulated = self.mouse_delta.get_or_insert_with(MousePosition::default);
                    accumulated.x += movement.x;
                    accumulated.y += movement.y;
                }
                self.last_mouse_position = Some(current);
            }

            WindowEvent::MouseWheel { delta, .. } => match delta {
                MouseScrollDelta::LineDelta(_, vertical) => {
                    self.mouse_wheel_delta += vertical;
                }
                MouseScrollDelta::PixelDelta(position) => {
                    self.mouse_wheel_delta += (position.y / SCROLL_PIXELS_PER_LINE) as f32;
                }
            },

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
