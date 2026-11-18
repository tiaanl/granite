use winit::event::WindowEvent;

use crate::SurfaceSize;

/// System notifications.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SystemEvent {
    /// The window's client area changed size, in physical pixels.
    Resized(SurfaceSize),
    /// The window's display scale factor changed.
    ScaleFactorChanged { scale_factor: f64 },
}

impl SystemEvent {
    /// Convert resize and scale notifications, ignoring all other window events.
    pub fn from_window_event(event: &WindowEvent) -> Option<Self> {
        Some(match event {
            WindowEvent::Resized(size) => Self::Resized(SurfaceSize::new(size.width, size.height)),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => Self::ScaleFactorChanged {
                scale_factor: *scale_factor,
            },
            _ => return None,
        })
    }
}
