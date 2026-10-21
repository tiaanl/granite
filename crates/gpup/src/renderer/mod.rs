//! Low-level window/surface renderer built directly on top of `wgpu`.
//!
//! [`Renderer`] owns the surface and produces a [`Frame`] containing the active
//! swapchain [`wgpu::TextureView`].
//!
//! For the higher-level draw-list/material layer, use the companion `gpup-draw` crate.

use std::sync::Arc;

use crate::SurfaceSize;

use thiserror::Error;
use winit::window::Window;

pub use frame::*;

mod frame;

#[derive(Debug, Error)]
/// Errors that can happen while creating a [`Renderer`].
pub enum RendererCreateError {
    /// Could not create a surface for the window.
    #[error("Could not create the window render surface. {0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),

    /// Could not determine a valid surface configuration.
    #[error("The window surface configuration could not be determined.")]
    DetermineConfiguration,

    /// Could not acquire a compatible graphics adapter.
    #[error("Could not request a graphics adapter. {0}")]
    RequestAdapter(#[from] wgpu::RequestAdapterError),

    /// Could not create a logical device and queue.
    #[error("Could not request a device and queue from the adapter. {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
}

#[derive(Debug, Error)]
/// Errors that can happen while submitting a frame.
pub enum NextFrameError {
    /// Could not acquire the current swapchain frame.
    #[error("Could not acquire the current frame from the render surface. {0}")]
    AcquireCurrentFrame(String),

    #[error("Could not recreate the surface. {0}")]
    CreateSurfaceError(#[from] wgpu::CreateSurfaceError),

    #[error("Surface validation error.")]
    ValidationError,
}

/// Low-level renderer that owns the window surface and exposes `wgpu` state.
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    instance: wgpu::Instance,
    window: Arc<Window>,
    needs_reconfigure: bool,

    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl Renderer {
    pub fn new(window: Arc<Window>) -> Result<Self, RendererCreateError> {
        let instance = wgpu::Instance::default();

        let winit::dpi::PhysicalSize { width, height } = window.inner_size();

        let surface = instance.create_surface(Arc::clone(&window))?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))?;

        let mut surface_config = surface
            .get_default_config(&adapter, width, height)
            .ok_or_else(|| RendererCreateError::DetermineConfiguration)?;
        surface_config.present_mode = wgpu::PresentMode::Fifo;

        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;

        if width > 0 && height > 0 {
            surface.configure(&device, &surface_config);
        }

        Ok(Self {
            surface,
            surface_config,
            instance,
            window,
            needs_reconfigure: false,
            device,
            queue,
        })
    }

    #[inline]
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    #[inline]
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    #[inline]
    pub fn surface_size(&self) -> SurfaceSize {
        SurfaceSize::new(self.surface_config.width, self.surface_config.height)
    }

    #[inline]
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.surface_config.format
    }

    pub fn resize(&mut self, size: SurfaceSize) {
        let SurfaceSize { width, height } = size;

        self.surface_config.width = width;
        self.surface_config.height = height;

        if width == 0 || height == 0 {
            return;
        }

        self.surface.configure(&self.device, &self.surface_config);
        self.needs_reconfigure = false;
    }

    /// Acquires the next swapchain texture, or skips the frame if unavailable.
    pub fn next_frame(&mut self) -> Result<Option<Frame>, NextFrameError> {
        if self.surface_config.width == 0 || self.surface_config.height == 0 {
            return Ok(None);
        }

        // Retry recovery once, without spinning if the surface stays unavailable.
        let surface_texture = 't: {
            for _ in 0..2 {
                if self.needs_reconfigure {
                    self.surface.configure(&self.device, &self.surface_config);
                    self.needs_reconfigure = false;
                }

                match self.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(texture) => {
                        break 't Some(texture);
                    }
                    wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                        // Use this frame; configure only after the caller releases it.
                        self.needs_reconfigure = true;
                        break 't Some(texture);
                    }
                    wgpu::CurrentSurfaceTexture::Timeout
                    | wgpu::CurrentSurfaceTexture::Occluded => {
                        break 't None;
                    }
                    wgpu::CurrentSurfaceTexture::Outdated => {
                        self.needs_reconfigure = true;
                    }
                    wgpu::CurrentSurfaceTexture::Lost => {
                        self.surface = self.instance.create_surface(Arc::clone(&self.window))?;
                        self.needs_reconfigure = true;
                    }
                    wgpu::CurrentSurfaceTexture::Validation => {
                        return Err(NextFrameError::ValidationError);
                    }
                }
            }

            // Both attempts exhausted without acquiring a texture.
            None
        };

        if let Some(surface_texture) = surface_texture {
            let view = surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let size = self.surface_size();
            let format = self.surface_config.format;

            Ok(Some(Frame::new(surface_texture, view, size, format)))
        } else {
            Ok(None)
        }
    }

    /// Present the given texture to the window surface.
    pub fn present(&self, frame: Frame) {
        self.window.pre_present_notify();
        self.queue.present(frame.surface_texture);
    }
}
