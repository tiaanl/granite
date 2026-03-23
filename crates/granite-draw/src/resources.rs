use glam::UVec2;
use wgpu::{self, util::DeviceExt};

use crate::{
    AsStorageBufferElement, AsUniformBuffer, BindGroupBindingResourceKey, BlendMode, DepthBufferId,
    DepthCompare, DrawListRenderer, FragmentShaderId, FrameContext, Material,
    MaterialDepthState, MaterialId, MaterialRecord, MeshId, RenderTargetId, SamplerId,
    ShaderModuleId, ShaderVisibility, StorageBufferId, StorageBufferRecord, TextureId, UniformId,
    UniformRecord, VertexShaderId,
    bindings::DrawBinding,
    common::Id,
    depth_buffer::{DepthBufferRecord, DepthBufferSize},
    draw_list::RenderTarget,
    encode_storage_buffer_elements,
    mesh::{AsVertexBufferLayout, Mesh},
    render_target::{RenderTargetFormat, RenderTargetRecord, RenderTargetSize},
    sampler::{SamplerAddressing, SamplerFiltering},
    storage_buffer_min_binding_size,
    textures::{TextureFormat, TextureRecord},
};

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

impl DrawListRenderer {
    /// Creates a [`Material`] directly from WGSL source.
    ///
    /// Leaves both entry points unspecified, so the shader's only `@vertex`
    /// and `@fragment` entry points are used automatically.
    pub fn create_material_from_shader(&mut self, name: &str, source: &str) -> Material {
        let shader = self.create_shader(name, source);
        let vertex_shader = self
            .vertex_shaders
            .push(VertexShader::create(shader, Option::<String>::None));
        let fragment_shader = self
            .fragment_shaders
            .push(FragmentShader::create(shader, Option::<String>::None));
        Material::new(vertex_shader, fragment_shader)
    }

    /// Creates a new depth buffer that can be attached by materials during drawing.
    ///
    /// No GPU texture is allocated at this point; allocation is deferred to the first clear or
    /// draw call that uses this depth buffer. Newly allocated depth buffers must be cleared before
    /// they can be loaded by a draw pass.
    ///
    /// The `size` parameter controls how the depth buffer's dimensions are determined:
    /// - [`DepthBufferSize::SurfaceSize`]: automatically tracks the render surface size.
    ///   Cannot be manually resized; resize happens automatically on first use after a window resize.
    /// - [`DepthBufferSize::Custom`]: fixed size managed manually via
    ///   [`DrawList::resize_depth_buffer`].
    pub fn create_depth_buffer(&mut self, name: &str, size: DepthBufferSize) -> DepthBufferId {
        let record = match size {
            DepthBufferSize::SurfaceSize => DepthBufferRecord::create_surface_sized(name),
            DepthBufferSize::Custom(s) => DepthBufferRecord::create_custom(name, s),
        };
        self.depth_buffers.push(record)
    }

    /// Recreates a depth buffer at a new size.
    ///
    /// Only valid for depth buffers created with [`DepthBufferSize::Custom`]. Calling this on
    /// a [`DepthBufferSize::SurfaceSize`] buffer logs a warning and does nothing; those buffers
    /// resize automatically when the surface is resized. After resizing, the depth buffer must be
    /// cleared again before it can be used for drawing.
    pub fn resize_depth_buffer(&mut self, id: DepthBufferId, size: UVec2) {
        let Some(record) = self.depth_buffers.get(id) else {
            tracing::warn!("resize_depth_buffer: invalid depth buffer id ({id:?})");
            return;
        };
        if matches!(record.size_mode, DepthBufferSize::SurfaceSize) {
            tracing::warn!(
                "resize_depth_buffer: depth buffer ({id:?}) uses SurfaceSize and resizes \
                 automatically; manual resize is not allowed"
            );
            return;
        }
        self.resize_depth_buffer_unchecked(id, size);
    }

    /// Ensures a depth buffer's GPU texture is allocated and up to date before a clear or draw
    /// call.
    ///
    /// Returns `true` when the backing texture was allocated or reallocated.
    pub(super) fn ensure_depth_buffer_ready(
        &mut self,
        frame_context: &FrameContext<'_>,
        depth_buffer: DepthBufferId,
    ) -> bool {
        let Some(record) = self.depth_buffers.get(depth_buffer) else {
            tracing::warn!("ensure_depth_buffer_ready: invalid depth buffer id ({depth_buffer:?})");
            return false;
        };

        let needs_allocation = match record.size_mode {
            DepthBufferSize::SurfaceSize => {
                record.view.is_none() || record.size != frame_context.size
            }
            DepthBufferSize::Custom(_) => record.view.is_none(),
        };

        if !needs_allocation {
            return false;
        }

        let size = match record.size_mode {
            DepthBufferSize::SurfaceSize => frame_context.size,
            DepthBufferSize::Custom(s) => s,
        };

        if let Some(record) = self.depth_buffers.get_mut(depth_buffer) {
            record.allocate(&self.device, size);
            return true;
        }

        false
    }

    pub(super) fn render_target_size(
        &self,
        surface_size: UVec2,
        render_target: RenderTarget,
    ) -> Option<UVec2> {
        match render_target {
            RenderTarget::Surface => Some(surface_size),
            RenderTarget::Custom(id) => Some(self.render_targets.get(id)?.size),
        }
    }

    /// Creates a new render target that can be drawn into and later bound as a texture.
    ///
    /// No GPU texture is allocated at this point; allocation is deferred to the first draw call
    /// that uses this target.
    ///
    /// The `size` parameter controls how the render target's dimensions are determined:
    /// - [`RenderTargetSize::SurfaceSize`]: automatically tracks the render surface size.
    ///   Cannot be manually resized; resize happens automatically on first use after a window resize.
    /// - [`RenderTargetSize::Custom`]: fixed size managed manually via
    ///   [`DrawList::resize_render_target`].
    pub fn create_render_target(
        &mut self,
        name: &str,
        size: RenderTargetSize,
        format: RenderTargetFormat,
    ) -> RenderTargetId {
        let record = match size {
            RenderTargetSize::SurfaceSize => RenderTargetRecord::create_surface_sized(name, format),
            RenderTargetSize::Custom(s) => RenderTargetRecord::create_custom(name, s, format),
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
        if matches!(record.size_mode, RenderTargetSize::SurfaceSize) {
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
    pub(super) fn ensure_render_target_ready(
        &mut self,
        frame_context: &FrameContext<'_>,
        render_target: RenderTarget,
    ) {
        let RenderTarget::Custom(id) = render_target else {
            return;
        };
        let Some(record) = self.render_targets.get(id) else {
            return;
        };

        let needs_allocation = match record.size_mode {
            RenderTargetSize::SurfaceSize => {
                record.view.is_none() || record.size != frame_context.size
            }
            RenderTargetSize::Custom(_) => record.view.is_none(),
        };

        if !needs_allocation {
            return;
        }

        let size = match record.size_mode {
            RenderTargetSize::SurfaceSize => frame_context.size,
            RenderTargetSize::Custom(s) => s,
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

    pub(super) fn render_target_format(
        &self,
        surface_format: wgpu::TextureFormat,
        render_target: RenderTarget,
    ) -> Option<wgpu::TextureFormat> {
        match render_target {
            RenderTarget::Surface => Some(surface_format),
            RenderTarget::Custom(id) => Some(self.render_targets.get(id)?.format.to_wgpu()),
        }
    }

    /// Invalidates the GPU texture for a `Custom` depth buffer at a new size.
    ///
    /// The texture is not reallocated immediately; it is created lazily on the next draw call
    /// that uses this buffer.
    pub(super) fn resize_depth_buffer_unchecked(&mut self, id: DepthBufferId, size: UVec2) {
        let Some(record) = self.depth_buffers.get_mut(id) else {
            tracing::warn!("resize_depth_buffer: invalid depth buffer id ({id:?})");
            return;
        };

        record.size_mode = DepthBufferSize::Custom(size);
        record.size = size;
        record.initialized = false;
        record._texture = None;
        record.view = None;
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

        record.size_mode = RenderTargetSize::Custom(size);
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
    pub fn create_mesh<V: AsVertexBufferLayout>(
        &mut self,
        name: &str,
        vertices: &[V],
        indices: &[u32],
    ) -> MeshId {
        let vertex_buffer_layout_id = self.get_or_create_vertex_buffer_layout(V::layout());

        let mesh = Mesh::create(
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

    /// Registers a material and returns its handle.
    pub fn create_material(&mut self, material: Material) -> MaterialId {
        self.materials.push(MaterialRecord {
            vertex_shader: material.vertex_shader,
            fragment_shader: material.fragment_shader,
            bindings: material.bindings,
            blend_mode: material.blend_mode,
            depth_state: material.depth_state,
        })
    }

    fn create_buffer_with_usage(
        &mut self,
        name: &str,
        data: &[u8],
        usage: wgpu::BufferUsages,
    ) -> Id {
        let buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(name),
                contents: data,
                usage,
            });

        self.buffers.push(buffer)
    }

    fn write_buffer_bytes(&self, buffer_id: Id, data: &[u8]) -> bool {
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
        let buffer = self.create_buffer_with_usage(
            &format!("{name}_uniform"),
            initial_bytes.as_slice(),
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
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
        self.write_buffer_bytes(uniform.buffer, data)
    }

    fn evict_storage_buffer_bind_groups(&mut self, id: StorageBufferId) {
        self.bind_groups.retain_keys(|key| {
            !key.bindings
                .iter()
                .any(|binding| binding.resource == BindGroupBindingResourceKey::StorageBuffer(id))
        });
    }

    /// Creates a read-only storage buffer array resource with initial elements.
    pub fn create_storage_buffer<T: AsStorageBufferElement>(
        &mut self,
        name: &str,
        initial_values: &[T],
    ) -> Option<StorageBufferId> {
        if initial_values.is_empty() {
            tracing::warn!("Could not create storage buffer `{name}` with zero elements.");
            return None;
        }

        let initial_bytes = encode_storage_buffer_elements(initial_values)
            .unwrap_or_else(|error| panic!("Could not encode storage buffer `{name}`: {error}"));
        let Ok(byte_len) = u64::try_from(initial_bytes.len()) else {
            tracing::warn!(
                "Could not create storage buffer `{name}`: byte length does not fit in u64."
            );
            return None;
        };

        let buffer = self.create_buffer_with_usage(
            &format!("{name}_storage"),
            initial_bytes.as_slice(),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        Some(self.storage_buffers.push(StorageBufferRecord {
            buffer,
            min_binding_size: storage_buffer_min_binding_size::<T>(),
            byte_len,
        }))
    }

    /// Creates a read-only storage buffer array resource from raw bytes.
    pub fn create_storage_buffer_bytes(
        &mut self,
        name: &str,
        min_binding_size: wgpu::BufferSize,
        data: &[u8],
    ) -> Option<StorageBufferId> {
        if data.is_empty() {
            tracing::warn!("Could not create storage buffer `{name}` with zero bytes.");
            return None;
        }

        let Ok(byte_len) = u64::try_from(data.len()) else {
            tracing::warn!(
                "Could not create storage buffer `{name}`: byte length does not fit in u64."
            );
            return None;
        };
        if byte_len < min_binding_size.get() {
            tracing::warn!(
                "Could not create storage buffer `{name}`: {} bytes is smaller than the declared minimum binding size {}.",
                byte_len,
                min_binding_size.get()
            );
            return None;
        }

        let buffer = self.create_buffer_with_usage(
            &format!("{name}_storage"),
            data,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        Some(self.storage_buffers.push(StorageBufferRecord {
            buffer,
            min_binding_size,
            byte_len,
        }))
    }

    /// Writes a complete array into an existing typed storage buffer.
    pub fn write_storage_buffer<T: AsStorageBufferElement>(
        &mut self,
        storage_buffer: StorageBufferId,
        data: &[T],
    ) -> bool {
        if data.is_empty() {
            tracing::warn!("Storage buffer write rejected for {storage_buffer:?}: zero elements.");
            return false;
        }

        let encoded = match encode_storage_buffer_elements(data) {
            Ok(encoded) => encoded,
            Err(error) => {
                tracing::warn!("Could not encode storage buffer for {storage_buffer:?}: {error}");
                return false;
            }
        };
        self.write_storage_buffer_bytes(storage_buffer, encoded.as_slice())
    }

    /// Writes raw bytes into an existing storage buffer.
    pub fn write_storage_buffer_bytes(
        &mut self,
        storage_buffer_id: StorageBufferId,
        data: &[u8],
    ) -> bool {
        let Some(storage_buffer) = self.storage_buffers.get(storage_buffer_id) else {
            tracing::warn!("Invalid storage buffer id ({storage_buffer_id:?})");
            return false;
        };
        let buffer_id = storage_buffer.buffer;
        let min_binding_size = storage_buffer.min_binding_size;
        let current_byte_len = storage_buffer.byte_len;

        if data.is_empty() {
            tracing::warn!("Storage buffer write rejected for {storage_buffer_id:?}: zero bytes.");
            return false;
        }

        let Ok(byte_len) = u64::try_from(data.len()) else {
            tracing::warn!(
                "Storage buffer write rejected for {storage_buffer_id:?}: byte length does not fit in u64."
            );
            return false;
        };
        if byte_len < min_binding_size.get() {
            tracing::warn!(
                "Storage buffer write size mismatch for {storage_buffer_id:?}: minimum binding size is {} bytes, got {} bytes.",
                min_binding_size.get(),
                byte_len
            );
            return false;
        }
        if byte_len == current_byte_len {
            return self.write_buffer_bytes(buffer_id, data);
        }

        self.evict_storage_buffer_bind_groups(storage_buffer_id);

        let Some(buffer) = self.buffers.get_mut(buffer_id) else {
            tracing::warn!(
                "Invalid buffer id ({:?}) for storage buffer ({storage_buffer_id:?})",
                buffer_id
            );
            return false;
        };
        *buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("storage_buffer"),
                contents: data,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        if let Some(storage_buffer) = self.storage_buffers.get_mut(storage_buffer_id) {
            storage_buffer.byte_len = byte_len;
            return true;
        }

        tracing::warn!("Invalid storage buffer id ({storage_buffer_id:?})");
        false
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
            mipmap_filter: wgpu::FilterMode::Nearest,
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

impl Material {
    /// Creates a new material for the given vertex and fragment shaders.
    pub fn new(vertex_shader: VertexShaderId, fragment_shader: FragmentShaderId) -> Self {
        Self {
            vertex_shader,
            fragment_shader,
            bindings: Vec::new(),
            blend_mode: BlendMode::default(),
            depth_state: None,
        }
    }

    fn push_binding(mut self, binding: DrawBinding) -> Self {
        self.bindings.push(binding);
        self
    }

    /// Adds a uniform binding at `@group(group) @binding(binding)`.
    pub fn uniform(self, group: u32, binding: u32, uniform: UniformId) -> Self {
        self.push_binding(DrawBinding::uniform(group, binding, uniform))
    }

    /// Adds a storage buffer binding at `@group(group) @binding(binding)`.
    pub fn storage_buffer(
        self,
        group: u32,
        binding: u32,
        storage_buffer: StorageBufferId,
        visibility: ShaderVisibility,
    ) -> Self {
        self.push_binding(DrawBinding::storage_buffer(
            group,
            binding,
            storage_buffer,
            visibility,
        ))
    }

    /// Adds a texture binding at `@group(group) @binding(binding)`.
    pub fn texture(self, group: u32, binding: u32, texture: TextureId) -> Self {
        self.push_binding(DrawBinding::texture(group, binding, texture))
    }

    /// Adds a render target as a texture binding at `@group(group) @binding(binding)`.
    pub fn render_target_texture(
        self,
        group: u32,
        binding: u32,
        render_target: RenderTargetId,
    ) -> Self {
        self.push_binding(DrawBinding::render_target(group, binding, render_target))
    }

    /// Adds a sampler binding at `@group(group) @binding(binding)`.
    pub fn sampler(self, group: u32, binding: u32, sampler: SamplerId) -> Self {
        self.push_binding(DrawBinding::sampler(group, binding, sampler))
    }

    /// Sets the blending mode for this material. Defaults to [`BlendMode::AlphaBlend`].
    pub fn blend_mode(mut self, blend_mode: BlendMode) -> Self {
        self.blend_mode = blend_mode;
        self
    }

    /// Attaches a depth buffer to this material and enables depth writes.
    pub fn depth_buffer(mut self, depth_buffer: DepthBufferId, compare: DepthCompare) -> Self {
        self.depth_state = Some(MaterialDepthState {
            depth_buffer,
            compare,
            write_enabled: true,
        });
        self
    }

    /// Attaches a depth buffer to this material with explicit depth-write control.
    pub fn depth_buffer_with_write(
        mut self,
        depth_buffer: DepthBufferId,
        compare: DepthCompare,
        write_enabled: bool,
    ) -> Self {
        self.depth_state = Some(MaterialDepthState {
            depth_buffer,
            compare,
            write_enabled,
        });
        self
    }
}
