//! Low-level window/surface renderer built directly on top of `wgpu`.
//!
//! [`Renderer`] owns the surface and produces a [`Frame`] containing the active
//! swapchain [`wgpu::TextureView`].
//!
//! For the higher-level draw-list/material layer, use the companion `granite-draw` crate.

use std::sync::Arc;

use glam::UVec2;
use thiserror::Error;
use winit::window::Window;

pub use frame::*;

mod frame;

#[derive(Debug, Error)]
/// Errors that can happen while creating a [`Renderer`].
pub enum RendererCreateError {
    /// Could not create a surface for the window.
    #[error("Could not create the window render surface! ({0})")]
    CreateSurface(String),

    /// Could not determine a valid surface configuration.
    #[error("The window surface configuration could not be determined!")]
    DetermineConfigurtation,

    /// Could not acquire a compatible graphics adapter.
    #[error("Could not request a graphics adapter! ({0})")]
    RequestAdapter(String),

    /// Could not create a logical device and queue.
    #[error("Could not request a device and queue from the adapter! ({0})")]
    RequestDevice(String),
}

#[derive(Debug, Error)]
/// Errors that can happen while submitting a frame.
pub enum SubmitFrameError {
    /// Could not acquire the current swapchain frame.
    #[error("Could not acquire the current frame from the render surface! ({0})")]
    AcquireCurrentFrame(String),
}

/// Low-level renderer that owns the window surface and exposes `wgpu` state.
pub struct Renderer {
    pub _adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
}

impl Renderer {
    /// Creates a new renderer for a window and initial surface size.
    pub fn new(window: Arc<Window>, size: UVec2) -> Result<Self, RendererCreateError> {
        let instance = wgpu::Instance::default();

        let surface = instance
            .create_surface(window)
            .map_err(|error| RendererCreateError::CreateSurface(error.to_string()))?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .map_err(|error| RendererCreateError::RequestAdapter(error.to_string()))?;

        let Some(surface_config) = surface
            .get_default_config(&adapter, size.x.max(1), size.y.max(1))
            .or(surface.get_configuration())
        else {
            return Err(RendererCreateError::DetermineConfigurtation);
        };

        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .map_err(|error| RendererCreateError::RequestDevice(error.to_string()))?;

        surface.configure(&device, &surface_config);

        Ok(Self {
            _adapter: adapter,
            device,
            queue,
            surface,
            surface_config,
        })
    }

    /// Get the current surface size.
    pub fn surface_size(&self) -> UVec2 {
        UVec2::new(self.surface_config.width, self.surface_config.height)
    }

    /// Get the format of the underlying texture.
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.surface_config.format
    }

    /// Resizes and reconfigures the surface.
    pub fn resize(&mut self, size: UVec2) {
        self.surface_config.width = size.x.max(1);
        self.surface_config.height = size.y.max(1);

        self.surface.configure(&self.device, &self.surface_config);
    }

    /// Acquires the next surface texture for a new frame.
    pub fn begin_frame(&mut self) -> Result<Frame, SubmitFrameError> {
        let surface_texture = self.get_current_surface_texture()?;
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        Ok(Frame::new(
            view,
            surface_texture,
            self.surface_size(),
            self.surface_format(),
        ))
    }

    /// Presents the frame after any externally submitted command buffers have completed encoding.
    pub fn submit_frame(&self, frame: Frame) {
        frame.present();
    }

    fn get_current_surface_texture(&mut self) -> Result<wgpu::SurfaceTexture, SubmitFrameError> {
        match self.surface.get_current_texture() {
            Ok(current_texture) => Ok(current_texture),

            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                tracing::warn!("Render surface lost or outdated, reconfiguring and retrying.");
                self.surface.configure(&self.device, &self.surface_config);

                match self.surface.get_current_texture() {
                    Ok(current_texture) => Ok(current_texture),

                    Err(error) => {
                        tracing::warn!(
                            "Could not acquire render surface texture after reconfigure! ({error})"
                        );
                        Err(SubmitFrameError::AcquireCurrentFrame(error.to_string()))
                    }
                }
            }

            Err(error) => {
                tracing::warn!("Could not acquire render surface texture! ({error})");
                Err(SubmitFrameError::AcquireCurrentFrame(error.to_string()))
            }
        }
    }
}
