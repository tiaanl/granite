//! A camera that represents a viewport into the world.

use glam::{Mat4, Quat, Vec3};

pub struct Camera {
    // The position of the camera in world space.
    pub translation: Vec3,
    // The rotation of the camera.
    pub rotation: Quat,

    /// The aspect ratio of the camera. Normally dependant on the aspect ratio of the window.
    pub aspect: f32,

    /// The calculated projection matrix.
    matrices: CameraMatrices,
}

impl Camera {
    pub fn new(translation: Vec3, rotation: Quat) -> Self {
        Self {
            translation,
            rotation,
            aspect: 1.0,
            matrices: CameraMatrices::default(),
        }
    }

    /// Move the camera forward by the distance specified.
    pub fn move_forward(&mut self, distance: f32) {
        self.translation += self.rotation * Vec3::Z * distance;
    }

    /// Move the camera right by the distance specified.
    pub fn move_right(&mut self, distance: f32) {
        self.translation += self.rotation * Vec3::X * distance;
    }

    /// Move the camera up by the distance specified.
    pub fn move_up(&mut self, distance: f32) {
        self.translation += self.rotation * Vec3::Y * distance;
    }

    /// Update the internal matrices.
    pub fn update(&mut self) {
        // TODO: Only update matrices if the inputs changed?
        self.matrices.projection =
            Mat4::perspective_rh(45.0_f32.to_radians(), self.aspect, 0.1, 10.0);
        self.matrices.view =
            Mat4::from_rotation_translation(self.rotation, self.translation).inverse();
    }
}

#[derive(Clone, Copy, Debug, Default, bytemuck::NoUninit)]
#[repr(C)]
pub struct CameraMatrices {
    projection: Mat4,
    view: Mat4,
}

pub struct GpuCamera {
    pub buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl GpuCamera {
    pub fn new(device: &wgpu::Device) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera_buffer"),
            size: std::mem::size_of::<CameraMatrices>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera_bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(buffer.as_entire_buffer_binding()),
            }],
        });

        Self {
            buffer,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn upload(&self, queue: &wgpu::Queue, camera: &Camera) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[camera.matrices]));
    }
}
