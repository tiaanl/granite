use crate::common::Id;

use super::*;
use wgpu::util::DeviceExt;

pub(super) struct ShaderModule {
    pub shader_module: wgpu::ShaderModule,
}

impl ShaderModule {
    fn create(device: &wgpu::Device, name: &str, source: &str) -> Self {
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{name}_module")),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(source)),
        });

        Self { shader_module }
    }
}

pub(super) struct VertexShader {
    pub shader_module: ShaderModuleId,
    pub entry_point: String,
}

impl VertexShader {
    fn create(shader_module: ShaderModuleId, entry_point: impl Into<String>) -> Self {
        Self {
            shader_module,
            entry_point: entry_point.into(),
        }
    }
}

pub(super) struct FragmentShader {
    pub shader_module: ShaderModuleId,
    pub entry_point: String,
}

impl FragmentShader {
    fn create(shader_module: ShaderModuleId, entry_point: impl Into<String>) -> Self {
        Self {
            shader_module,
            entry_point: entry_point.into(),
        }
    }
}

impl Renderer {
    /// Creates a mesh resource and returns a stable mesh handle.
    pub fn create_mesh<V: mesh::AsVertexBufferLayout>(
        &mut self,
        name: &str,
        vertices: &[V],
        indices: &[u32],
    ) -> MeshId {
        let vertex_buffer_layout_id = self.get_or_create_vertex_buffer_layout(V::layout());

        let mesh = mesh::Mesh::create(
            &self.device,
            name,
            vertex_buffer_layout_id,
            vertices,
            indices,
        );
        self.meshes.push(mesh)
    }

    /// Creates a WGSL shader module from source text.
    pub fn create_shader(&mut self, name: &str, source: &str) -> ShaderModuleId {
        let shader = ShaderModule::create(&self.device, name, source);
        self.shaders.push(shader)
    }

    /// Creates a vertex shader entry-point reference from a shader module.
    pub fn create_vertex_shader(
        &mut self,
        shader: ShaderModuleId,
        entry_point: impl Into<String>,
    ) -> VertexShaderId {
        let vertex_shader = VertexShader::create(shader, entry_point);
        self.vertex_shaders.push(vertex_shader)
    }

    /// Creates a fragment shader entry-point reference from a shader module.
    pub fn create_fragment_shader(
        &mut self,
        shader: ShaderModuleId,
        entry_point: impl Into<String>,
    ) -> FragmentShaderId {
        let fragment_shader = FragmentShader::create(shader, entry_point);
        self.fragment_shaders.push(fragment_shader)
    }

    /// Starts building a material for the provided vertex/fragment shaders.
    pub fn create_material(
        &mut self,
        vertex_shader: VertexShaderId,
        fragment_shader: FragmentShaderId,
    ) -> MaterialBuilder<'_> {
        MaterialBuilder::new(self, vertex_shader, fragment_shader)
    }

    fn insert_material(
        &mut self,
        vertex_shader: VertexShaderId,
        fragment_shader: FragmentShaderId,
        bindings: &[bindings::DrawBinding],
    ) -> MaterialId {
        self.materials.push(MaterialRecord {
            vertex_shader,
            fragment_shader,
            bindings: bindings.to_vec(),
        })
    }

    fn create_uniform_buffer<T: bytemuck::NoUninit>(&mut self, name: &str, data: &T) -> Id {
        let buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(name),
                contents: bytemuck::cast_slice(std::slice::from_ref(data)),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        self.buffers.push(buffer)
    }

    fn write_uniform_buffer_bytes(&self, buffer_id: Id, data: &[u8]) -> bool {
        let Some(buffer) = self.buffers.get(buffer_id) else {
            tracing::warn!("Invalid buffer id ({buffer_id:?})");
            return false;
        };

        self.queue.write_buffer(buffer, 0, data);
        true
    }

    /// Creates a uniform buffer resource with an initial value.
    pub fn create_uniform<T: AsUniformBuffer>(
        &mut self,
        name: &str,
        initial_value: &T,
    ) -> UniformId {
        let buffer = self.create_uniform_buffer(&format!("{name}_uniform"), initial_value);
        self.uniforms.push(UniformRecord {
            buffer,
            visibility: T::VISIBILITY,
            byte_len: std::mem::size_of::<T>(),
        })
    }

    #[allow(dead_code)]
    /// Writes a complete value into an existing uniform buffer.
    pub fn write_uniform<T: AsUniformBuffer>(&self, uniform: UniformId, data: &T) -> bool {
        self.write_uniform_bytes(uniform, bytemuck::cast_slice(std::slice::from_ref(data)))
    }

    pub(super) fn write_uniform_bytes(&self, uniform_id: UniformId, data: &[u8]) -> bool {
        let Some(uniform) = self.uniforms.get(uniform_id) else {
            tracing::warn!("Invalid uniform id ({uniform_id:?})");
            return false;
        };
        if data.len() != uniform.byte_len {
            tracing::warn!(
                "Uniform write size mismatch for {uniform_id:?}: expected {} bytes, got {} bytes.",
                uniform.byte_len,
                data.len()
            );
            return false;
        }
        self.write_uniform_buffer_bytes(uniform.buffer, data)
    }

    /// Creates an RGBA8 sRGB 2D texture and uploads pixel data.
    pub fn create_texture_rgba8(
        &mut self,
        name: &str,
        size: UVec2,
        data: &[u8],
    ) -> Option<TextureId> {
        if size.x == 0 || size.y == 0 {
            tracing::warn!("Cannot create texture with zero dimensions.");
            return None;
        }

        let expected_size = (size.x as usize) * (size.y as usize) * 4;
        if data.len() != expected_size {
            tracing::warn!(
                "Texture data size mismatch. Expected {expected_size} bytes, got {} bytes.",
                data.len()
            );
            return None;
        }

        let texture = self.device.create_texture_with_data(
            &self.queue,
            &wgpu::TextureDescriptor {
                label: Some(&format!("{name}_texture")),
                size: wgpu::Extent3d {
                    width: size.x,
                    height: size.y,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            data,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let texture_id = self.textures.push(TextureRecord {
            _texture: texture,
            view,
        });

        Some(texture_id)
    }

    /// Creates a default linear sampler.
    pub fn create_sampler(&mut self, name: &str) -> SamplerId {
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&format!("{name}_sampler")),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        self.samplers.push(sampler)
    }

    pub(super) fn create_bind_group_layout(
        &mut self,
        name: &str,
        entries: &[wgpu::BindGroupLayoutEntry],
    ) -> Id {
        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some(name),
                    entries,
                });

        self.bind_group_layouts.push(bind_group_layout)
    }
}

impl<'a> MaterialBuilder<'a> {
    fn new(
        renderer: &'a mut Renderer,
        vertex_shader: VertexShaderId,
        fragment_shader: FragmentShaderId,
    ) -> Self {
        Self {
            renderer,
            vertex_shader,
            fragment_shader,
            bindings: Vec::new(),
        }
    }

    fn push_binding(mut self, binding: bindings::DrawBinding) -> Self {
        self.bindings.push(binding);
        self
    }

    /// Adds a uniform binding at `@group(group) @binding(binding)`.
    pub fn uniform(self, group: u32, binding: u32, uniform: UniformId) -> Self {
        self.push_binding(bindings::DrawBinding::uniform(group, binding, uniform))
    }

    /// Adds a texture binding at `@group(group) @binding(binding)`.
    pub fn texture(self, group: u32, binding: u32, texture: TextureId) -> Self {
        self.push_binding(bindings::DrawBinding::texture(group, binding, texture))
    }

    /// Adds a sampler binding at `@group(group) @binding(binding)`.
    pub fn sampler(self, group: u32, binding: u32, sampler: SamplerId) -> Self {
        self.push_binding(bindings::DrawBinding::sampler(group, binding, sampler))
    }

    /// Finalizes and stores the material, returning its handle.
    pub fn build(self) -> MaterialId {
        let Self {
            renderer,
            vertex_shader,
            fragment_shader,
            bindings,
        } = self;
        renderer.insert_material(vertex_shader, fragment_shader, bindings.as_slice())
    }
}
