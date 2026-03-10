//! Higher-level draw-list renderer that layers on top of [`granite::renderer::Renderer`].
//!
//! `granite` owns the window surface and frame lifecycle. This crate provides stable resource
//! handles, materials, meshes, render targets, and draw-list submission into
//! [`granite::renderer::Frame`].

use std::collections::HashMap;

use granite::{renderer::Renderer, wgpu};

pub use encase;

use crate::{
    common::{Id, StableMap, StableSet, StableVec},
    mesh::VertexBufferLayout,
};

mod bindings;
mod commands;
mod common;
pub mod draw_list;
mod execution;
pub mod mesh;
mod prepared_draw;
pub mod render_target;
mod resources;
pub mod sampler;
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

/// Trait implemented by types that can be uploaded as uniforms.
pub trait AsUniformBuffer: crate::encase::ShaderType + crate::encase::internal::WriteInto {
    /// Shader stage visibility of this uniform.
    const VISIBILITY: ShaderVisibility;

    /// Minimum binding size required for this uniform.
    fn min_binding_size() -> wgpu::BufferSize {
        <Self as crate::encase::ShaderType>::min_size()
    }

    /// Encodes this uniform into GPU-ready bytes.
    fn encode_bytes(&self) -> crate::encase::internal::Result<Vec<u8>> {
        encode_uniform_bytes(self)
    }
}

fn encode_uniform_bytes<T: AsUniformBuffer + ?Sized>(
    uniform: &T,
) -> crate::encase::internal::Result<Vec<u8>> {
    let mut buffer = crate::encase::UniformBuffer::new(Vec::new());
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
    pub fn as_wgpu(self) -> wgpu::ShaderStages {
        match self {
            Self::Vertex => wgpu::ShaderStages::VERTEX,
            Self::Fragment => wgpu::ShaderStages::FRAGMENT,
            Self::VertexFragment => wgpu::ShaderStages::VERTEX_FRAGMENT,
            Self::Compute => wgpu::ShaderStages::COMPUTE,
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct PipelineLayoutKey {
    bind_group_layouts: Vec<Id>,
}

#[must_use]
/// Fluent material builder for adding uniform/texture/sampler bindings.
pub struct MaterialBuilder<'a> {
    renderer: &'a mut DrawListRenderer,
    vertex_shader: VertexShaderId,
    fragment_shader: FragmentShaderId,
    bindings: Vec<bindings::DrawBinding>,
    blend_mode: BlendMode,
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
    RenderTarget(RenderTargetId),
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

/// Blending behavior applied to a material's color output.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum BlendMode {
    /// No blending; source color fully replaces the destination.
    Opaque,
    /// Standard alpha blending: `src_alpha * src + (1 - src_alpha) * dst`.
    #[default]
    AlphaBlend,
    /// Additive blending: `src + dst`.
    Additive,
    /// Premultiplied alpha blending.
    Premultiplied,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct RenderPipelineKey {
    render_target_format: wgpu::TextureFormat,
    vertex_buffer_layout: Option<Id>,
    instance_buffer_layout: Option<Id>,
    pipeline_layout: Id,
    vertex_shader: VertexShaderId,
    fragment_shader: FragmentShaderId,
    blend_mode: BlendMode,
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
    blend_mode: BlendMode,
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

/// Higher-level draw-list layer with stable resource handles and cached pipelines.
pub struct DrawListRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,

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

impl DrawListRenderer {
    /// Creates a higher-level draw-list renderer on top of the provided [`Renderer`].
    pub fn new(renderer: &Renderer) -> Self {
        Self {
            device: renderer.device.clone(),
            queue: renderer.queue.clone(),
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
        }
    }
}
