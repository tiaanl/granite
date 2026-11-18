use winit::{
    event::{ElementState, MouseScrollDelta, WindowEvent},
    keyboard::PhysicalKey,
};

pub use winit::{event::MouseButton, keyboard::KeyCode};

// Conversion for consistent wheel and touchpad scrolling.
const SCROLL_PIXELS_PER_LINE: f64 = 40.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputEvent {
    /// A physical key was pressed.
    KeyDown {
        key: KeyCode,
        repeat: bool,
        is_synthetic: bool,
    },
    /// A physical key was released.
    KeyUp { key: KeyCode },
    /// The window lost keyboard focus.
    FocusLost,
    /// The mouse moved, in physical pixels relative to the window.
    MouseMove { position: MousePosition },
    /// A mouse button was pressed.
    MouseDown { button: MouseButton },
    /// A mouse button was released.
    MouseUp { button: MouseButton },
    /// Vertical scroll in lines.
    MouseScroll { delta: f32 },
    /// The cursor left the window.
    MouseLeft,
}

impl InputEvent {
    pub fn from_window_event(event: &WindowEvent) -> Option<Self> {
        Some(match event {
            WindowEvent::Focused(false) => Self::FocusLost,
            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => {
                let PhysicalKey::Code(key) = event.physical_key else {
                    return None;
                };
                match event.state {
                    ElementState::Pressed => Self::KeyDown {
                        key,
                        repeat: event.repeat,
                        is_synthetic: *is_synthetic,
                    },
                    ElementState::Released => Self::KeyUp { key },
                }
            }
            WindowEvent::CursorMoved { position, .. } => Self::MouseMove {
                position: MousePosition::new(position.x.round() as i32, position.y.round() as i32),
            },
            WindowEvent::CursorLeft { .. } => Self::MouseLeft,
            WindowEvent::MouseInput { state, button, .. } => match state {
                ElementState::Pressed => Self::MouseDown { button: *button },
                ElementState::Released => Self::MouseUp { button: *button },
            },
            WindowEvent::MouseWheel { delta, .. } => Self::MouseScroll {
                delta: match delta {
                    MouseScrollDelta::LineDelta(_, vertical) => *vertical,
                    MouseScrollDelta::PixelDelta(position) => {
                        (position.y / SCROLL_PIXELS_PER_LINE) as f32
                    }
                },
            },
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct MousePosition {
    pub x: i32,
    pub y: i32,
}

impl MousePosition {
    pub fn new(x: i32, y: i32) -> Self {
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
