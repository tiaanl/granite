use wgpu::util::DeviceExt;

use crate::common::Id;

/// Public alias for the vertex format type used by layout descriptions.
pub type VertexFormat = wgpu::VertexFormat;
/// Public alias for the vertex step mode used by buffer layouts.
pub type VertexStepMode = wgpu::VertexStepMode;

/// Describes how a vertex type maps to a GPU vertex buffer layout.
pub trait AsVertexBufferLayout: Sized + encase::ShaderSize + encase::internal::WriteInto {
    /// Byte stride of each element.
    const STRIDE: u64;
    /// Step mode used when advancing this buffer.
    const STEP_MODE: VertexStepMode = VertexStepMode::Vertex;
    /// Static list of vertex attributes for this type.
    const ATTRIBUTES: &'static [VertexAttribute];

    /// Returns the vertex buffer layout metadata for this type.
    fn layout() -> VertexBufferLayout {
        VertexBufferLayout {
            size: Self::STRIDE,
            step_mode: Self::STEP_MODE,
            attributes: Self::ATTRIBUTES.to_vec(),
        }
    }

    /// Encodes a full slice of values into GPU-ready vertex bytes.
    fn encode_slice(values: &[Self]) -> encase::internal::Result<Vec<u8>> {
        encode_struct_buffer_bytes(values)
    }
}

/// Describes how an instance type maps to a GPU instance buffer layout.
pub trait AsInstanceBufferLayout: Sized + encase::ShaderSize + encase::internal::WriteInto {
    /// Byte stride of each element.
    const STRIDE: u64;
    /// Step mode used when advancing this buffer.
    const STEP_MODE: VertexStepMode = VertexStepMode::Instance;
    /// Static list of vertex attributes for this type.
    const ATTRIBUTES: &'static [VertexAttribute];

    /// Returns the instance buffer layout metadata for this type.
    fn layout() -> VertexBufferLayout {
        VertexBufferLayout {
            size: Self::STRIDE,
            step_mode: Self::STEP_MODE,
            attributes: Self::ATTRIBUTES.to_vec(),
        }
    }

    /// Encodes a full slice of values into GPU-ready instance bytes.
    fn encode_slice(values: &[Self]) -> encase::internal::Result<Vec<u8>> {
        encode_struct_buffer_bytes(values)
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
/// One vertex attribute entry in a buffer layout.
pub struct VertexAttribute {
    /// GPU format of the attribute.
    pub format: VertexFormat,
    /// Byte offset from the start of each element.
    pub offset: u64,
}

#[derive(Clone, Hash, PartialEq, Eq)]
/// Compact description of a buffer layout used by the renderer pipeline cache.
pub struct VertexBufferLayout {
    /// Byte stride of each element.
    pub size: u64,
    /// GPU step mode for this buffer.
    pub step_mode: VertexStepMode,
    /// Ordered list of attributes in this layout.
    pub attributes: Vec<VertexAttribute>,
}

/// GPU mesh buffers and metadata used by draw commands.
pub struct Mesh {
    /// Layout handle used to resolve pipeline vertex state.
    pub vertex_buffer_layout_id: Id,
    /// Vertex buffer object.
    pub vertex_buffer: wgpu::Buffer,
    /// Index buffer object.
    pub index_buffer: wgpu::Buffer,
    /// Number of indices in the mesh.
    pub index_count: u32,
}

impl Mesh {
    /// Creates GPU buffers for a mesh from CPU vertex and index data.
    pub fn create<V: AsVertexBufferLayout>(
        device: &wgpu::Device,
        name: &str,
        vertex_buffer_layout_id: Id,
        vertices: &[V],
        indices: &[u32],
    ) -> Self {
        let vertex_bytes = V::encode_slice(vertices)
            .unwrap_or_else(|error| panic!("Could not encode vertex buffer `{name}`: {error}"));
        let index_bytes = encode_index_bytes(indices);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{name}_vertices")),
            contents: vertex_bytes.as_slice(),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{name}_indices")),
            contents: index_bytes.as_slice(),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            vertex_buffer_layout_id,
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        }
    }
}

pub(super) fn vertex_attributes(
    layout: &VertexBufferLayout,
    shader_location_offset: u32,
) -> Vec<wgpu::VertexAttribute> {
    let mut attributes = Vec::with_capacity(layout.attributes.len());

    for (shader_location, attr) in layout.attributes.iter().enumerate() {
        attributes.push(wgpu::VertexAttribute {
            format: attr.format,
            offset: attr.offset,
            shader_location: shader_location_offset + shader_location as u32,
        });
    }

    attributes
}

pub(super) fn encode_struct_buffer_bytes<T>(values: &[T]) -> encase::internal::Result<Vec<u8>>
where
    T: encase::ShaderSize + encase::internal::WriteInto,
{
    let stride = usize::try_from(T::SHADER_SIZE.get())
        .expect("vertex or instance element stride does not fit in usize");
    let total_size = stride
        .checked_mul(values.len())
        .expect("vertex or instance buffer size overflow");
    let mut bytes = Vec::with_capacity(total_size);

    for (index, value) in values.iter().enumerate() {
        let offset = index
            .checked_mul(stride)
            .expect("vertex or instance buffer offset overflow");
        let mut writer = encase::internal::Writer::new(value, &mut bytes, offset)?;
        value.write_into(&mut writer);
    }

    Ok(bytes)
}

fn encode_index_bytes(indices: &[u32]) -> Vec<u8> {
    let capacity = indices
        .len()
        .checked_mul(std::mem::size_of::<u32>())
        .expect("index buffer size overflow");
    let mut bytes = Vec::with_capacity(capacity);

    for index in indices {
        bytes.extend_from_slice(&index.to_le_bytes());
    }

    bytes
}
