use std::collections::HashSet;

use winit::event::WindowEvent;

use super::input::{InputEvent, MousePosition};

pub use winit::{event::MouseButton, keyboard::KeyCode};

/// Optional keyboard and mouse state accumulated from [`InputEvent`]s.
///
/// Store this in your scene, forward events to [`Self::handle_input_event`], and
/// call [`Self::reset_current_frame`] after consuming input in each update.
///
/// ```
/// use gpup::{Frame, InputEvent, InputState, Renderer, Scene};
///
/// #[derive(Default)]
/// struct MyScene {
///     input: InputState,
/// }
///
/// impl Scene for MyScene {
///     fn input(&mut self, event: &InputEvent) {
///         self.input.handle_input_event(event);
///     }
///
///     fn update(&mut self, delta_time: f32) {
///         // Read accumulated input here before clearing per-frame data.
///         self.input.reset_current_frame();
///     }
///
///     fn frame(&mut self, renderer: &Renderer, frame: &Frame) {}
/// }
/// ```
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
    /// Convenience adapter for applications receiving winit window events directly.
    pub fn handle_window_event(&mut self, event: &WindowEvent) {
        if let Some(event) = InputEvent::from_window_event(event) {
            self.handle_input_event(&event);
        }
    }

    /// Accumulate an input event. Frame data persists until explicitly reset.
    pub fn handle_input_event(&mut self, event: &InputEvent) {
        match event {
            InputEvent::FocusLost => {
                // Cancel input without generating release actions on focus loss.
                *self = Self::default();
            }
            InputEvent::MouseLeft => {
                // Preserve held inputs, but avoid a movement jump on re-entry.
                self.last_mouse_position = None;
                self.mouse_delta = None;
            }
            InputEvent::KeyDown {
                key,
                repeat,
                is_synthetic,
            } => {
                let newly_pressed = self.key_pressed.insert(*key);
                if newly_pressed && !repeat && !is_synthetic {
                    self.key_just_pressed.insert(*key);
                }
            }
            InputEvent::KeyUp { key } => {
                self.key_pressed.remove(key);
            }
            InputEvent::MouseDown { button } => {
                self.mouse_pressed.insert(*button);
                self.mouse_just_pressed.insert(*button);
            }
            InputEvent::MouseUp { button } => {
                self.mouse_pressed.remove(button);
                self.mouse_just_released.insert(*button);
            }
            InputEvent::MouseMove { position } => {
                if let Some(previous) = self.last_mouse_position {
                    let movement = *position - previous;
                    let accumulated = self.mouse_delta.get_or_insert_with(MousePosition::default);
                    accumulated.x += movement.x;
                    accumulated.y += movement.y;
                }
                self.last_mouse_position = Some(*position);
            }
            InputEvent::MouseScroll { delta } => {
                self.mouse_wheel_delta += delta;
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use winit::{
        dpi::PhysicalPosition,
        event::{DeviceId, ElementState, Ime, MouseScrollDelta, TouchPhase},
    };

    #[test]
    fn converts_basic_input_and_ignores_other_events() {
        let device_id = DeviceId::dummy();
        for (delta, expected) in [
            (MouseScrollDelta::LineDelta(10.0, 1.5), 1.5),
            (
                MouseScrollDelta::PixelDelta(PhysicalPosition::new(12.5, -80.0)),
                -2.0,
            ),
        ] {
            assert_eq!(
                InputEvent::from_window_event(&WindowEvent::MouseWheel {
                    device_id,
                    delta,
                    phase: TouchPhase::Moved,
                }),
                Some(InputEvent::MouseScroll { delta: expected }),
            );
        }
        assert_eq!(
            InputEvent::from_window_event(&WindowEvent::CursorMoved {
                device_id,
                position: PhysicalPosition::new(12.6, 20.2),
            }),
            Some(InputEvent::MouseMove {
                position: MousePosition::new(13, 20)
            }),
        );
        for (state, expected) in [
            (
                ElementState::Pressed,
                InputEvent::MouseDown {
                    button: MouseButton::Left,
                },
            ),
            (
                ElementState::Released,
                InputEvent::MouseUp {
                    button: MouseButton::Left,
                },
            ),
        ] {
            assert_eq!(
                InputEvent::from_window_event(&WindowEvent::MouseInput {
                    device_id,
                    state,
                    button: MouseButton::Left,
                }),
                Some(expected),
            );
        }
        assert_eq!(
            InputEvent::from_window_event(&WindowEvent::Focused(false)),
            Some(InputEvent::FocusLost)
        );
        assert_eq!(
            InputEvent::from_window_event(&WindowEvent::CursorLeft { device_id }),
            Some(InputEvent::MouseLeft)
        );
        for event in [
            WindowEvent::Focused(true),
            WindowEvent::Ime(Ime::Commit("hello".into())),
            WindowEvent::RedrawRequested,
            WindowEvent::CloseRequested,
        ] {
            assert_eq!(InputEvent::from_window_event(&event), None);
        }
    }

    #[test]
    fn mouse_leaving_preserves_held_inputs_and_restarts_position_tracking() {
        let mut input = InputState::default();
        input.handle_input_event(&InputEvent::KeyDown {
            key: KeyCode::KeyA,
            repeat: false,
            is_synthetic: false,
        });
        input.handle_input_event(&InputEvent::MouseDown {
            button: MouseButton::Left,
        });
        for position in [MousePosition::new(10, 20), MousePosition::new(15, 25)] {
            input.handle_input_event(&InputEvent::MouseMove { position });
        }
        assert_eq!(input.mouse_delta(), Some(MousePosition::new(5, 5)));

        input.handle_window_event(&WindowEvent::CursorLeft {
            device_id: DeviceId::dummy(),
        });
        assert_eq!(input.mouse_position(), None);
        assert_eq!(input.mouse_delta(), None);
        assert!(input.key_pressed(KeyCode::KeyA));
        assert!(input.mouse_pressed(MouseButton::Left));
        assert!(!input.mouse_just_released(MouseButton::Left));

        input.handle_input_event(&InputEvent::MouseMove {
            position: MousePosition::new(100, 200),
        });
        assert_eq!(input.mouse_position(), Some(MousePosition::new(100, 200)));
        assert_eq!(input.mouse_delta(), None);
        input.handle_input_event(&InputEvent::MouseMove {
            position: MousePosition::new(103, 204),
        });
        assert_eq!(input.mouse_delta(), Some(MousePosition::new(3, 4)));
    }

    #[test]
    fn key_state_preserves_repeat_and_synthetic_behavior() {
        let mut input = InputState::default();
        let press = InputEvent::KeyDown {
            key: KeyCode::KeyA,
            repeat: false,
            is_synthetic: false,
        };
        input.handle_input_event(&press);
        assert!(input.key_pressed(KeyCode::KeyA));
        assert!(input.key_just_pressed(KeyCode::KeyA));
        input.reset_current_frame();
        input.handle_input_event(&press);
        assert!(!input.key_just_pressed(KeyCode::KeyA));
        input.handle_input_event(&InputEvent::KeyUp { key: KeyCode::KeyA });
        assert!(!input.key_pressed(KeyCode::KeyA));

        for (repeat, is_synthetic) in [(true, false), (false, true)] {
            input.handle_input_event(&InputEvent::KeyDown {
                key: KeyCode::KeyA,
                repeat,
                is_synthetic,
            });
            assert!(input.key_pressed(KeyCode::KeyA));
            assert!(!input.key_just_pressed(KeyCode::KeyA));
            input.handle_input_event(&InputEvent::KeyUp { key: KeyCode::KeyA });
        }
    }

    #[test]
    fn accumulates_input_until_frame_reset_and_cancels_on_focus_loss() {
        let mut input = InputState::default();
        for position in [
            MousePosition::new(10, 20),
            MousePosition::new(14, 18),
            MousePosition::new(17, 25),
        ] {
            input.handle_input_event(&InputEvent::MouseMove { position });
        }
        input.handle_input_event(&InputEvent::MouseScroll { delta: 1.5 });
        input.handle_input_event(&InputEvent::MouseScroll { delta: 2.0 });
        input.handle_input_event(&InputEvent::MouseDown {
            button: MouseButton::Left,
        });
        input.handle_input_event(&InputEvent::KeyDown {
            key: KeyCode::KeyA,
            repeat: false,
            is_synthetic: false,
        });
        assert_eq!(input.mouse_delta(), Some(MousePosition::new(7, 5)));
        assert_eq!(input.mouse_wheel_delta(), 3.5);
        assert!(input.mouse_just_pressed(MouseButton::Left));

        input.reset_current_frame();
        assert_eq!(input.mouse_delta(), None);
        assert_eq!(input.mouse_wheel_delta(), 0.0);
        assert_eq!(input.mouse_position(), Some(MousePosition::new(17, 25)));
        assert!(input.mouse_pressed(MouseButton::Left));
        assert!(!input.mouse_just_pressed(MouseButton::Left));

        input.handle_input_event(&InputEvent::MouseUp {
            button: MouseButton::Left,
        });
        assert!(!input.mouse_pressed(MouseButton::Left));
        assert!(input.mouse_just_released(MouseButton::Left));
        input.reset_current_frame();
        assert!(!input.mouse_just_released(MouseButton::Left));

        input.handle_input_event(&InputEvent::MouseDown {
            button: MouseButton::Left,
        });
        input.handle_input_event(&InputEvent::FocusLost);
        assert!(!input.key_pressed(KeyCode::KeyA));
        assert!(!input.mouse_pressed(MouseButton::Left));
        assert!(!input.mouse_just_pressed(MouseButton::Left));
        assert!(!input.mouse_just_released(MouseButton::Left));
        assert_eq!(input.mouse_position(), None);
    }
}
