use crate::{
    common::Id,
    renderer::textures::{TextureFormat, TextureRecord},
};

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
    pub entry_point: Option<String>,
}

impl VertexShader {
    fn create(shader_module: ShaderModuleId, entry_point: Option<impl Into<String>>) -> Self {
        Self {
            shader_module,
            entry_point: entry_point.map(Into::into),
        }
    }
}

pub(super) struct FragmentShader {
    pub shader_module: ShaderModuleId,
    pub entry_point: Option<String>,
}

impl FragmentShader {
    fn create(shader_module: ShaderModuleId, entry_point: Option<impl Into<String>>) -> Self {
        Self {
            shader_module,
            entry_point: entry_point.map(Into::into),
        }
    }
}

impl Renderer {
    /// Creates a material builder directly from WGSL source.
    ///
    /// Leaves both entry points unspecified, so the shader's only `@vertex`
    /// and `@fragment` entry points are used automatically.
    pub fn create_material_from_shader(&mut self, name: &str, source: &str) -> MaterialBuilder<'_> {
        let shader = self.create_shader(name, source);
        let vertex_shader = self
            .vertex_shaders
            .push(VertexShader::create(shader, Option::<String>::None));
        let fragment_shader = self
            .fragment_shaders
            .push(FragmentShader::create(shader, Option::<String>::None));
        self.create_material(vertex_shader, fragment_shader)
    }

    /// Creates a new render target that can be drawn into and later bound as a texture.
    ///
    /// No GPU texture is allocated at this point; allocation is deferred to the first draw call
    /// that uses this target.
    ///
    /// The `size` parameter controls how the render target's dimensions are determined:
    /// - [`RenderTargetSize::SurfaceSize`]: automatically tracks the render surface size.
    ///   Cannot be manually resized; resize happens automatically on first use after a window resize.
    /// - [`RenderTargetSize::Custom`]: fixed size managed manually via [`Frame::resize_render_target`].
    pub fn create_render_target(
        &mut self,
        name: &str,
        size: RenderTargetSize,
        format: RenderTargetFormat,
    ) -> RenderTargetId {
        let record = match size {
            RenderTargetSize::SurfaceSize => {
                render_target::RenderTargetRecord::create_surface_sized(name, format)
            }
            RenderTargetSize::Custom(s) => {
                render_target::RenderTargetRecord::create_custom(name, s, format)
            }
        };
        self.render_targets.push(record)
    }

    /// Recreates a render target at a new size, keeping the same format.
    ///
    /// Only valid for render targets created with [`RenderTargetSize::Custom`]. Calling this on
    /// a [`RenderTargetSize::SurfaceSize`] target logs a warning and does nothing; those targets
    /// resize automatically when the surface is resized.
    ///
    /// Only bind groups that sampled this specific render target are evicted;
    /// all others remain cached. Evicted bind groups are lazily recreated on
    /// the next draw call.
    pub fn resize_render_target(&mut self, id: RenderTargetId, size: UVec2) {
        let Some(record) = self.render_targets.get(id) else {
            tracing::warn!("resize_render_target: invalid render target id ({id:?})");
            return;
        };
        if matches!(record.size_mode, render_target::RenderTargetSize::SurfaceSize) {
            tracing::warn!(
                "resize_render_target: render target ({id:?}) uses SurfaceSize and resizes \
                 automatically; manual resize is not allowed"
            );
            return;
        }
        self.resize_render_target_unchecked(id, size);
    }

    /// Ensures a render target's GPU texture is allocated and up to date before a draw call.
    ///
    /// - `SurfaceSize`: allocates if not yet created, or reallocates if the surface was resized.
    /// - `Custom`: allocates if not yet created. Size only changes via an explicit
    ///   `resize_render_target` call, which invalidates the texture; it is reallocated here on
    ///   next use.
    /// - `RenderTarget::Surface`: no-op.
    pub(super) fn ensure_render_target_ready(&mut self, render_target: RenderTarget) {
        let RenderTarget::Custom(id) = render_target else {
            return;
        };
        let Some(record) = self.render_targets.get(id) else {
            return;
        };

        let needs_allocation = match record.size_mode {
            render_target::RenderTargetSize::SurfaceSize => {
                record.view.is_none() || record.size != self.surface_size()
            }
            render_target::RenderTargetSize::Custom(_) => record.view.is_none(),
        };

        if !needs_allocation {
            return;
        }

        let size = match record.size_mode {
            render_target::RenderTargetSize::SurfaceSize => self.surface_size(),
            render_target::RenderTargetSize::Custom(s) => s,
        };

        // Evict stale bind groups before reallocating, since the old TextureView is going away.
        self.bind_groups.retain_keys(|key| {
            !key.bindings
                .iter()
                .any(|b| b.resource == BindGroupBindingResourceKey::RenderTarget(id))
        });

        if let Some(record) = self.render_targets.get_mut(id) {
            record.allocate(&self.device, size);
        }
    }

    /// Invalidates the GPU texture for a `Custom` render target at a new size.
    ///
    /// The texture is not reallocated immediately; it is created lazily on the next draw call
    /// that uses this target. Stale bind groups are evicted now so they are recreated on next use.
    pub(super) fn resize_render_target_unchecked(&mut self, id: RenderTargetId, size: UVec2) {
        let Some(record) = self.render_targets.get_mut(id) else {
            tracing::warn!("resize_render_target: invalid render target id ({id:?})");
            return;
        };

        record.size_mode = render_target::RenderTargetSize::Custom(size);
        record.size = size;
        record._texture = None;
        record.view = None;

        // Evict bind groups referencing the now-invalid TextureView.
        self.bind_groups.retain_keys(|key| {
            !key.bindings
                .iter()
                .any(|b| b.resource == BindGroupBindingResourceKey::RenderTarget(id))
        });
    }

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
        let vertex_shader = VertexShader::create(shader, Some(entry_point));
        self.vertex_shaders.push(vertex_shader)
    }

    /// Creates a fragment shader entry-point reference from a shader module.
    pub fn create_fragment_shader(
        &mut self,
        shader: ShaderModuleId,
        entry_point: impl Into<String>,
    ) -> FragmentShaderId {
        let fragment_shader = FragmentShader::create(shader, Some(entry_point));
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
        blend_mode: BlendMode,
    ) -> MaterialId {
        self.materials.push(MaterialRecord {
            vertex_shader,
            fragment_shader,
            bindings: bindings.to_vec(),
            blend_mode,
        })
    }

    fn create_uniform_buffer(&mut self, name: &str, data: &[u8]) -> Id {
        let buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(name),
                contents: data,
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
        let initial_bytes = initial_value
            .encode_bytes()
            .unwrap_or_else(|error| panic!("Could not encode uniform `{name}`: {error}"));
        let buffer =
            self.create_uniform_buffer(&format!("{name}_uniform"), initial_bytes.as_slice());
        self.uniforms.push(UniformRecord {
            buffer,
            visibility: T::VISIBILITY,
            min_binding_size: T::min_binding_size(),
        })
    }

    /// Writes a complete value into an existing uniform buffer.
    pub fn write_uniform<T: AsUniformBuffer>(&self, uniform: UniformId, data: &T) -> bool {
        let encoded = match data.encode_bytes() {
            Ok(encoded) => encoded,
            Err(error) => {
                tracing::warn!("Could not encode uniform for {uniform:?}: {error}");
                return false;
            }
        };
        self.write_uniform_bytes(uniform, encoded.as_slice())
    }

    pub(super) fn write_uniform_bytes(&self, uniform_id: UniformId, data: &[u8]) -> bool {
        let Some(uniform) = self.uniforms.get(uniform_id) else {
            tracing::warn!("Invalid uniform id ({uniform_id:?})");
            return false;
        };
        let Ok(expected_byte_len) = usize::try_from(uniform.min_binding_size.get()) else {
            tracing::warn!(
                "Uniform {uniform_id:?} has unsupported min binding size {} for this platform.",
                uniform.min_binding_size.get()
            );
            return false;
        };
        if data.len() != expected_byte_len {
            tracing::warn!(
                "Uniform write size mismatch for {uniform_id:?}: expected {} bytes, got {} bytes.",
                expected_byte_len,
                data.len()
            );
            return false;
        }
        self.write_uniform_buffer_bytes(uniform.buffer, data)
    }

    /// Create a new texture with the pixels given.
    pub fn create_texture(
        &mut self,
        name: &str,
        size: UVec2,
        format: TextureFormat,
        data: &[u8],
    ) -> Option<TextureId> {
        if size.x == 0 || size.y == 0 {
            tracing::warn!("Cannot create texture with zero dimensions.");
            return None;
        }

        let expected_size = (size.x as usize) * (size.y as usize) * format.bytes_per_pixel();
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
                format: format.to_wgpu(),
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            data,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let texture_id = self.textures.push(TextureRecord {
            _texture: texture,
            format,
            view,
            size,
        });

        Some(texture_id)
    }

    pub(super) fn write_texture_rgba8_region(
        &self,
        texture_id: TextureId,
        origin: UVec2,
        size: UVec2,
        data: &[u8],
    ) -> bool {
        if size.x == 0 || size.y == 0 {
            tracing::warn!("Texture partial write rejected: zero-sized region.");
            return false;
        }

        let Some(texture) = self.textures.get(texture_id) else {
            tracing::warn!("Invalid texture id ({texture_id:?})");
            return false;
        };

        let Some(end_x) = origin.x.checked_add(size.x) else {
            tracing::warn!("Texture partial write rejected: x-range overflow.");
            return false;
        };
        let Some(end_y) = origin.y.checked_add(size.y) else {
            tracing::warn!("Texture partial write rejected: y-range overflow.");
            return false;
        };
        if end_x > texture.size.x || end_y > texture.size.y {
            tracing::warn!(
                "Texture partial write out of bounds for {texture_id:?}: texture={}x{}, region=({}, {})..({}, {}).",
                texture.size.x,
                texture.size.y,
                origin.x,
                origin.y,
                end_x,
                end_y
            );
            return false;
        }

        let expected_size =
            (size.x as usize) * (size.y as usize) * texture.format.bytes_per_pixel();
        if data.len() != expected_size {
            tracing::warn!(
                "Texture partial write size mismatch for {texture_id:?}: expected {expected_size} bytes, got {} bytes.",
                data.len()
            );
            return false;
        }

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture._texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: origin.x,
                    y: origin.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size.x * texture.format.bytes_per_pixel() as u32),
                rows_per_image: Some(size.y),
            },
            wgpu::Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
        );
        true
    }

    /// Creates a sampler with addressing/filtering options.
    pub fn create_sampler(
        &mut self,
        name: &str,
        addressing: SamplerAddressing,
        filtering: SamplerFiltering,
    ) -> SamplerId {
        let address_mode: wgpu::AddressMode = addressing.into();
        let filter_mode: wgpu::FilterMode = filtering.into();

        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&format!("{name}_sampler")),
            address_mode_u: address_mode,
            address_mode_v: address_mode,
            address_mode_w: address_mode,
            mag_filter: filter_mode,
            min_filter: filter_mode,
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
            blend_mode: BlendMode::default(),
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

    /// Adds a render target as a texture binding at `@group(group) @binding(binding)`.
    pub fn render_target_texture(
        self,
        group: u32,
        binding: u32,
        render_target: RenderTargetId,
    ) -> Self {
        self.push_binding(bindings::DrawBinding::render_target(
            group,
            binding,
            render_target,
        ))
    }

    /// Adds a sampler binding at `@group(group) @binding(binding)`.
    pub fn sampler(self, group: u32, binding: u32, sampler: SamplerId) -> Self {
        self.push_binding(bindings::DrawBinding::sampler(group, binding, sampler))
    }

    /// Sets the blending mode for this material. Defaults to [`BlendMode::AlphaBlend`].
    pub fn blend_mode(mut self, blend_mode: BlendMode) -> Self {
        self.blend_mode = blend_mode;
        self
    }

    /// Finalizes and stores the material, returning its handle.
    pub fn build(self) -> MaterialId {
        let Self {
            renderer,
            vertex_shader,
            fragment_shader,
            bindings,
            blend_mode,
        } = self;
        renderer.insert_material(
            vertex_shader,
            fragment_shader,
            bindings.as_slice(),
            blend_mode,
        )
    }
}
