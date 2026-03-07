//! A small `wgpu` renderer with stable resource handles.
//!
//! # Important Notes
//! - Resource handles are stable [`Id`] values and are only valid for the [`Renderer`] that created
//!   them.
//! - IDs must not be reused across different renderer instances.
//! - Rendering is command-buffer style: build a frame command list and submit it once per frame.
//! - [`Renderer::submit_frame`] can fail when the surface is temporarily unavailable (for example
//!   surface lost/outdated); the renderer already reconfigures internally for that case, so the
//!   caller can usually just try again next frame.
//!
//! # Usage Example
//! ```ignore
//! use std::sync::Arc;
//! use glam::UVec2;
//! use renderer::Renderer;
//!
//! fn render_once(window: Arc<winit::window::Window>) -> Result<(), Box<dyn std::error::Error>> {
//!     let mut renderer = Renderer::new(window, UVec2::new(1280, 720))?;
//!
//!     let shader = renderer.create_shader("main", "...wgsl...");
//!     let vs = renderer.create_vertex_shader(shader, "vertex");
//!     let fs = renderer.create_fragment_shader(shader, "fragment");
//!     let material = renderer.create_material(vs, fs).build();
//!
//!     // Add mesh data to the renderer.
//!     let vertices = &[/* your vertex data */];
//!     let indices: &[u32] = &[/* your index data */];
//!     let mesh = renderer.create_mesh("quad", vertices, indices);
//!
//!     // Queue an indexed draw call (via instanced draw API).
//!     let instances = &[/* your instance data */];
//!     let mut frame = renderer.begin_frame();
//!     frame.draw_instanced(mesh, material, instances);
//!     renderer.submit_frame(frame)?;
//!     Ok(())
//! }
//! ```
use std::{collections::HashMap, sync::Arc};

use glam::UVec2;
use thiserror::Error;
use winit::window::Window;

pub use frame::*;
pub use mesh::*;

pub use render_target::RenderTargetFormat;
pub use sampler::*;
pub use textures::TextureFormat;

use crate::common::{Id, StableMap, StableSet, StableVec};

mod bindings;
mod commands;
mod execution;
mod frame;
mod mesh;
mod prepared_draw;
mod render_target;
mod resources;
mod sampler;
mod textures;

/// Handle to a uniform resource.
pub type UniformId = Id;
/// Handle to a texture resource.
pub type TextureId = Id;
/// Handle to a sampler resource.
pub type SamplerId = Id;
/// Handle to a material resource.
pub type MaterialId = Id;
/// Handle to a mesh resource.
pub type MeshId = Id;
/// Handle to a render target resource.
pub type RenderTargetId = Id;
/// Handle to a shader module resource.
pub type ShaderModuleId = Id;
/// Handle to a vertex shader entry-point resource.
pub type VertexShaderId = Id;
/// Handle to a fragment shader entry-point resource.
pub type FragmentShaderId = Id;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct PipelineLayoutKey {
    bind_group_layouts: Vec<Id>,
}

/// Trait implemented by types that can be uploaded as uniforms.
pub trait AsUniformBuffer: encase::ShaderType + encase::internal::WriteInto {
    /// Shader stage visibility of this uniform.
    const VISIBILITY: ShaderVisibility;
}

fn encode_uniform_bytes<T: AsUniformBuffer>(uniform: &T) -> encase::internal::Result<Vec<u8>> {
    let mut buffer = encase::UniformBuffer::new(Vec::new());
    buffer.write(uniform)?;
    Ok(buffer.into_inner())
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[allow(dead_code)]
/// Shader stage visibility flags for bindings.
pub enum ShaderVisibility {
    /// Visible to vertex shaders.
    Vertex,
    /// Visible to fragment shaders.
    Fragment,
    /// Visible to both vertex and fragment shaders.
    VertexFragment,
    /// Visible to compute shaders.
    Compute,
}

impl ShaderVisibility {
    fn as_wgpu(self) -> wgpu::ShaderStages {
        match self {
            Self::Vertex => wgpu::ShaderStages::VERTEX,
            Self::Fragment => wgpu::ShaderStages::FRAGMENT,
            Self::VertexFragment => wgpu::ShaderStages::VERTEX_FRAGMENT,
            Self::Compute => wgpu::ShaderStages::COMPUTE,
        }
    }
}

#[must_use]
/// Fluent material builder for adding uniform/texture/sampler bindings.
pub struct MaterialBuilder<'a> {
    renderer: &'a mut Renderer,
    vertex_shader: VertexShaderId,
    fragment_shader: FragmentShaderId,
    bindings: Vec<bindings::DrawBinding>,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
enum BindGroupLayoutBindingTypeKey {
    Uniform,
    Texture,
    Sampler,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct BindGroupLayoutBindingKey {
    binding: u32,
    visibility: ShaderVisibility,
    ty: BindGroupLayoutBindingTypeKey,
    min_binding_size: Option<wgpu::BufferSize>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct BindGroupLayoutKey {
    bindings: Vec<BindGroupLayoutBindingKey>,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
enum BindGroupBindingResourceKey {
    Uniform(UniformId),
    Texture(TextureId),
    Sampler(SamplerId),
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct BindGroupBindingKey {
    binding: u32,
    resource: BindGroupBindingResourceKey,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct BindGroupKey {
    bind_group_layout: Id,
    bindings: Vec<BindGroupBindingKey>,
}

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

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct RenderPipelineKey {
    render_target: RenderTarget,
    vertex_buffer_layout: Option<Id>,
    instance_buffer_layout: Option<Id>,
    pipeline_layout: Id,
    vertex_shader: VertexShaderId,
    fragment_shader: FragmentShaderId,
}

struct BindGroupRecord {
    bind_group: wgpu::BindGroup,
}

struct UniformRecord {
    buffer: Id,
    visibility: ShaderVisibility,
    min_binding_size: wgpu::BufferSize,
}

struct MaterialRecord {
    vertex_shader: VertexShaderId,
    fragment_shader: FragmentShaderId,
    bindings: Vec<bindings::DrawBinding>,
}

struct ResolvedDrawBindGroup {
    slot: u32,
    bind_group: Id,
    bind_group_layout: Id,
}

struct ResolvedDrawBindings {
    bind_groups_to_set: Vec<ResolvedDrawBindGroup>,
    pipeline_layout_key: PipelineLayoutKey,
}

/// Main renderer object that owns GPU resources and render state.
pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,

    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,

    render_targets: StableVec<render_target::RenderTargetRecord>,
    vertex_buffer_layouts: StableSet<VertexBufferLayout>,
    instance_buffer_layouts: StableSet<VertexBufferLayout>,
    bind_group_layouts: StableMap<BindGroupLayoutKey, wgpu::BindGroupLayout>,
    bind_groups: StableMap<BindGroupKey, BindGroupRecord>,
    buffers: StableVec<wgpu::Buffer>,
    uniforms: StableVec<UniformRecord>,
    textures: StableVec<textures::TextureRecord>,
    samplers: StableVec<wgpu::Sampler>,
    materials: StableVec<MaterialRecord>,
    pipeline_layouts: StableMap<PipelineLayoutKey, wgpu::PipelineLayout>,
    meshes: StableVec<mesh::Mesh>,
    shaders: StableVec<resources::ShaderModule>,
    vertex_shaders: StableVec<resources::VertexShader>,
    fragment_shaders: StableVec<resources::FragmentShader>,

    empty_bind_group_layout: Option<Id>,
    render_pipeline_cache: HashMap<RenderPipelineKey, wgpu::RenderPipeline>,
}

// Plumbing.
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
            device,
            queue,
            surface,
            surface_config,

            render_targets: StableVec::default(),
            vertex_buffer_layouts: StableSet::default(),
            instance_buffer_layouts: StableSet::default(),
            bind_group_layouts: StableMap::default(),
            bind_groups: StableMap::default(),
            buffers: StableVec::default(),
            uniforms: StableVec::default(),
            textures: StableVec::default(),
            samplers: StableVec::default(),
            materials: StableVec::default(),
            pipeline_layouts: StableMap::default(),
            meshes: StableVec::default(),
            shaders: StableVec::default(),
            vertex_shaders: StableVec::default(),
            fragment_shaders: StableVec::default(),

            empty_bind_group_layout: None,
            render_pipeline_cache: HashMap::default(),
        })
    }

    /// Get the current surface size.
    pub fn surface_size(&self) -> UVec2 {
        UVec2::new(self.surface_config.width, self.surface_config.height)
    }

    /// Resizes and reconfigures the surface.
    pub fn resize(&mut self, size: UVec2) {
        self.surface_config.width = size.x.max(1);
        self.surface_config.height = size.y.max(1);

        self.surface.configure(&self.device, &self.surface_config);
    }
}
