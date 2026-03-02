use super::{AsInstanceBufferLayout, AsUniformBuffer, MaterialId, MeshId, UniformId, commands};

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
        mesh: MeshId,
        material: MaterialId,
        instances: &[I],
    ) {
        if instances.is_empty() {
            return;
        }

        self.commands.push(commands::FrameCommand::DrawIndexed(
            commands::DrawIndexedCommand {
                mesh,
                material,
                instance_buffer_layout: I::layout(),
                instance_data: bytemuck::cast_slice(instances).to_vec(),
                instance_count: instances.len() as u32,
            },
        ));
    }
}
