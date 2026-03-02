use wgpu::util::DeviceExt;

use crate::common::Id;

/// Public alias for the vertex format type used by layout descriptions.
pub type VertexFormat = wgpu::VertexFormat;

/// Describes how a vertex type maps to a GPU vertex buffer layout.
pub trait AsVertexBufferLayout: Sized + bytemuck::NoUninit {
    /// Returns the vertex buffer layout metadata for this type.
    fn layout() -> VertexBufferLayout;
}

/// Describes how an instance type maps to a GPU instance buffer layout.
pub trait AsInstanceBufferLayout: Sized + bytemuck::NoUninit {
    /// Returns the instance buffer layout metadata for this type.
    fn layout() -> VertexBufferLayout;
}

#[derive(Clone, Hash, PartialEq, Eq)]
/// One vertex attribute entry in a buffer layout.
pub struct VertexAttribute {
    /// GPU format of the attribute.
    pub format: VertexFormat,
}

#[derive(Clone, Hash, PartialEq, Eq)]
/// Compact description of a buffer layout used by the renderer pipeline cache.
pub struct VertexBufferLayout {
    /// Byte stride of each element.
    pub size: u64,
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
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{name}_vertices")),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{name}_indices")),
            contents: bytemuck::cast_slice(indices),
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

    let mut offset = 0;

    for (shader_location, attr) in layout.attributes.iter().enumerate() {
        attributes.push(wgpu::VertexAttribute {
            format: attr.format,
            offset,
            shader_location: shader_location_offset + shader_location as u32,
        });

        offset += attr.format.size();
    }

    attributes
}
