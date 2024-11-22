use glam::{Vec2, Vec3};
use wgpu::util::DeviceExt;

pub struct GpuMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: Option<wgpu::Buffer>,
    vertex_or_index_count: u32,
}

#[derive(Clone, Copy, Debug, bytemuck::NoUninit)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub tex_coord: Vec2,
}

pub struct IndexedMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl IndexedMesh {
    pub fn to_gpu_mesh(self, device: &wgpu::Device) -> GpuMesh {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("world_vertex_buffer"),
            contents: bytemuck::cast_slice(&self.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("world_index_buffer"),
            contents: bytemuck::cast_slice(&self.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        GpuMesh {
            vertex_buffer,
            index_buffer: Some(index_buffer),
            vertex_or_index_count: self.indices.len() as u32,
        }
    }
}

pub struct TriMesh {
    pub vertices: Vec<Vertex>,
}

impl TriMesh {
    pub fn to_gpu_mesh(self, device: &wgpu::Device) -> GpuMesh {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("world_vertex_buffer"),
            contents: bytemuck::cast_slice(&self.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        GpuMesh {
            vertex_buffer,
            index_buffer: None,
            vertex_or_index_count: self.vertices.len() as u32,
        }
    }
}

pub trait RenderPassMeshExt {
    fn draw_mesh(&mut self, mesh: &GpuMesh);
}

impl<'encoder> RenderPassMeshExt for wgpu::RenderPass<'encoder> {
    fn draw_mesh(&mut self, mesh: &GpuMesh) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        if let Some(ref index_buffer) = mesh.index_buffer {
            self.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            self.draw_indexed(0..mesh.vertex_or_index_count, 0, 0..1);
        } else {
            self.draw(0..mesh.vertex_or_index_count, 0..1);
        }
    }
}

pub fn create_plane(size: f32) -> IndexedMesh {
    let hs = size / 2.0;
    let vertices = vec![
        Vertex {
            position: Vec3::new(-hs, -hs, 0.0),
            normal: Vec3::Z,
            tex_coord: Vec2::new(0.0, 0.0),
        },
        Vertex {
            position: Vec3::new(hs, -hs, 0.0),
            normal: Vec3::Z,
            tex_coord: Vec2::new(1.0, 0.0),
        },
        Vertex {
            position: Vec3::new(hs, hs, 0.0),
            normal: Vec3::Z,
            tex_coord: Vec2::new(1.0, 1.0),
        },
        Vertex {
            position: Vec3::new(-hs, hs, 0.0),
            normal: Vec3::Y,
            tex_coord: Vec2::new(0.0, 1.0),
        },
    ];

    let indices = vec![0, 1, 2, 2, 3, 0];

    IndexedMesh { vertices, indices }
}
