use crate::prelude::RenderTargetId;

use super::{AsInstanceBufferLayout, AsUniformBuffer, MaterialId, MeshId, UniformId, commands};

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
            commands::UniformUpdateCommand {
                uniform,
                data: bytemuck::cast_slice(std::slice::from_ref(data)).to_vec(),
            },
        ));
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

        self.commands.push(commands::FrameCommand::DrawIndexed(
            commands::DrawIndexedCommand {
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
