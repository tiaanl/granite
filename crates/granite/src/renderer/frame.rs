use crate::prelude::RenderTargetId;
use glam::UVec2;

use super::{
    AsInstanceBufferLayout, AsUniformBuffer, MaterialId, MeshId, TextureId, UniformId, commands,
};

/// Specify the render target for a draw command.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum RenderTarget {
    Surface,
    Custom(RenderTargetId),
}

/// Command list for a single frame to be submitted to the renderer.
#[derive(Default)]
pub struct Frame {
    pub(super) commands: Vec<commands::FrameCommand>,
}

impl Frame {
    /// Queues an update for a previously created uniform.
    pub fn update_uniform<T: AsUniformBuffer>(&mut self, uniform: UniformId, data: &T) {
        self.commands.push(commands::FrameCommand::UpdateUniform(
            commands::UpdateUniform {
                uniform,
                data: bytemuck::cast_slice(std::slice::from_ref(data)).to_vec(),
            },
        ));
    }

    /// Queues an update of a region of the specifed texture.
    pub fn update_texture_region(
        &mut self,
        texture: TextureId,
        origin: UVec2,
        size: UVec2,
        data: &[u8],
    ) {
        if size.x == 0 || size.y == 0 {
            return;
        }

        self.commands
            .push(commands::FrameCommand::UpdateTextureRegion(
                commands::UpdateTextureRegion {
                    texture,
                    origin,
                    size,
                    data: data.to_vec(),
                },
            ));
    }

    /// Queues an indexed draw using the provided mesh and material.
    pub fn draw_mesh(&mut self, render_target: RenderTarget, mesh: MeshId, material: MaterialId) {
        self.commands
            .push(commands::FrameCommand::DrawMesh(commands::DrawMesh {
                render_target,
                mesh,
                material,
            }));
    }

    /// Queues a non-indexed draw using only the material pipeline.
    pub fn draw(&mut self, render_target: RenderTarget, material: MaterialId, vertex_count: u32) {
        if vertex_count == 0 {
            return;
        }

        self.commands.push(commands::FrameCommand::Draw(commands::Draw {
            render_target,
            material,
            vertex_count,
        }));
    }

    /// Queues an instanced indexed draw using the provided mesh and material.
    pub fn draw_mesh_instanced<I: AsInstanceBufferLayout>(
        &mut self,
        render_target: RenderTarget,
        mesh: MeshId,
        material: MaterialId,
        instances: &[I],
    ) {
        if instances.is_empty() {
            return;
        }

        self.commands
            .push(commands::FrameCommand::DrawMeshInstanced(
                commands::DrawMeshInstanced {
                    render_target,
                    mesh,
                    material,
                    instance_buffer_layout: I::layout(),
                    instance_data: bytemuck::cast_slice(instances).to_vec(),
                    instance_count: instances.len() as u32,
                },
            ));
    }
}
