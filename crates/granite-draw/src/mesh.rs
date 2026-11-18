use crate::Id;
use wgpu::util::DeviceExt;

/// Public alias for the vertex format type used by layout descriptions.
pub type VertexFormat = wgpu::VertexFormat;
/// Public alias for the vertex step mode used by buffer layouts.
pub type VertexStepMode = wgpu::VertexStepMode;
/// Public alias for the index format type used by mesh index buffers.
pub type IndexFormat = wgpu::IndexFormat;

/// Trait implemented by index element types supported by [`MeshBuilder`].
pub trait AsIndexBufferElement: Copy {
    /// GPU index format for this element type.
    const FORMAT: IndexFormat;

    fn append_to_bytes(bytes: &mut Vec<u8>, value: Self);
}

impl AsIndexBufferElement for u16 {
    const FORMAT: IndexFormat = IndexFormat::Uint16;

    fn append_to_bytes(bytes: &mut Vec<u8>, value: Self) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
}

impl AsIndexBufferElement for u32 {
    const FORMAT: IndexFormat = IndexFormat::Uint32;

    fn append_to_bytes(bytes: &mut Vec<u8>, value: Self) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
}

/// Describes how a vertex type maps to a GPU vertex buffer layout.
pub trait AsVertexBufferLayout:
    Sized + crate::encase::ShaderSize + crate::encase::internal::WriteInto
{
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
    fn encode_slice(values: &[Self]) -> crate::encase::internal::Result<Vec<u8>> {
        encode_struct_buffer_bytes(values)
    }
}

/// Describes how an instance type maps to a GPU instance buffer layout.
pub trait AsInstanceBufferLayout:
    Sized + crate::encase::ShaderSize + crate::encase::internal::WriteInto
{
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
    fn encode_slice(values: &[Self]) -> crate::encase::internal::Result<Vec<u8>> {
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
/// Compact description of a buffer layout used by higher-level pipeline caches.
pub struct VertexBufferLayout {
    /// Byte stride of each element.
    pub size: u64,
    /// GPU step mode for this buffer.
    pub step_mode: VertexStepMode,
    /// Ordered list of attributes in this layout.
    pub attributes: Vec<VertexAttribute>,
}

#[derive(Clone)]
struct MeshVertexBufferData {
    layout: VertexBufferLayout,
    bytes: Vec<u8>,
    element_count: u32,
}

#[derive(Clone)]
struct MeshIndexBufferData {
    format: IndexFormat,
    bytes: Vec<u8>,
    index_count: u32,
}

/// Builder describing a mesh with zero or more vertex buffers and an optional index buffer.
#[derive(Clone, Default)]
pub struct MeshBuilder {
    vertex_buffers: Vec<MeshVertexBufferData>,
    index_buffer: Option<MeshIndexBufferData>,
    vertex_count: Option<u32>,
}

impl MeshBuilder {
    /// Creates an empty mesh description.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a typed vertex buffer to the mesh.
    pub fn with_vertex_buffer<V: AsVertexBufferLayout>(mut self, values: &[V]) -> Self {
        let bytes = V::encode_slice(values)
            .unwrap_or_else(|error| panic!("Could not encode mesh vertex buffer: {error}"));

        self.vertex_buffers.push(MeshVertexBufferData {
            layout: V::layout(),
            bytes,
            element_count: values.len() as u32,
        });
        self
    }

    /// Attaches an index buffer to the mesh.
    ///
    /// Supports `u16` and `u32` indices.
    pub fn with_index_buffer<I: AsIndexBufferElement>(mut self, indices: &[I]) -> Self {
        self.index_buffer = Some(MeshIndexBufferData {
            format: I::FORMAT,
            bytes: encode_index_bytes(indices),
            index_count: indices.len() as u32,
        });
        self
    }

    /// Overrides the vertex count used for non-indexed draws.
    pub fn with_vertex_count(mut self, vertex_count: u32) -> Self {
        self.vertex_count = Some(vertex_count);
        self
    }

    pub(super) fn vertex_buffer_layouts(&self) -> impl Iterator<Item = VertexBufferLayout> + '_ {
        self.vertex_buffers
            .iter()
            .map(|buffer| buffer.layout.clone())
    }

    fn vertex_count(&self) -> u32 {
        self.vertex_count.unwrap_or_else(|| {
            self.vertex_buffers
                .first()
                .map_or(0, |buffer| buffer.element_count)
        })
    }
}

pub(super) struct MeshIndexBuffer {
    pub buffer: wgpu::Buffer,
    pub format: IndexFormat,
    pub index_count: u32,
}

pub(super) enum MeshDrawCount {
    Indexed { index_count: u32 },
    NonIndexed { vertex_count: u32 },
}

pub(super) struct Mesh {
    pub vertex_buffer_layout_ids: Vec<Id>,
    pub vertex_buffers: Vec<wgpu::Buffer>,
    pub index_buffer: Option<MeshIndexBuffer>,
    pub vertex_count: u32,
}

impl Mesh {
    pub(super) fn create(
        device: &wgpu::Device,
        name: &str,
        builder: MeshBuilder,
        vertex_buffer_layout_ids: Vec<Id>,
    ) -> Self {
        let vertex_count = builder.vertex_count();
        let MeshBuilder {
            vertex_buffers,
            index_buffer,
            vertex_count: _,
        } = builder;

        debug_assert_eq!(vertex_buffer_layout_ids.len(), vertex_buffers.len());

        let mut gpu_vertex_buffers = Vec::with_capacity(vertex_buffers.len());
        for (buffer_index, vertex_buffer) in vertex_buffers.into_iter().enumerate() {
            gpu_vertex_buffers.push(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{name}_vertices_{buffer_index}")),
                    contents: vertex_buffer.bytes.as_slice(),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                }),
            );
        }

        let gpu_index_buffer = index_buffer.map(|index_buffer| MeshIndexBuffer {
            buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{name}_indices")),
                contents: index_buffer.bytes.as_slice(),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            }),
            format: index_buffer.format,
            index_count: index_buffer.index_count,
        });

        Self {
            vertex_buffer_layout_ids,
            vertex_buffers: gpu_vertex_buffers,
            index_buffer: gpu_index_buffer,
            vertex_count,
        }
    }

    pub(super) fn draw_count(&self) -> MeshDrawCount {
        if let Some(index_buffer) = &self.index_buffer {
            MeshDrawCount::Indexed {
                index_count: index_buffer.index_count,
            }
        } else {
            MeshDrawCount::NonIndexed {
                vertex_count: self.vertex_count,
            }
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

fn encode_index_bytes<I: AsIndexBufferElement>(indices: &[I]) -> Vec<u8> {
    let capacity = indices
        .len()
        .checked_mul(std::mem::size_of::<I>())
        .expect("index buffer size overflow");
    let mut bytes = Vec::with_capacity(capacity);

    for index in indices {
        I::append_to_bytes(&mut bytes, *index);
    }

    bytes
}

fn encode_struct_buffer_bytes<T>(values: &[T]) -> crate::encase::internal::Result<Vec<u8>>
where
    T: crate::encase::ShaderSize + crate::encase::internal::WriteInto,
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
        let mut writer = crate::encase::internal::Writer::new(value, &mut bytes, offset)?;
        value.write_into(&mut writer);
    }

    Ok(bytes)
}
