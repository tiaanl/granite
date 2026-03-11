use glam::UVec2;

use crate::{
    AsUniformBuffer, MaterialId, MeshId, RenderTargetId, TextureId, UniformId,
    commands::{
        Draw, DrawMesh, DrawMeshInstanced, FrameCommand, ResizeRenderTarget, UpdateTextureRegion,
        UpdateUniform,
    },
    mesh::AsInstanceBufferLayout,
};

/// Specify the render target for a draw command.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum RenderTarget {
    /// Draw directly to the window surface.
    Surface,
    /// Draw to a custom off-screen render target.
    Custom(RenderTargetId),
}

/// Recorded draw and upload commands for a single submission.
#[derive(Default)]
pub struct DrawList {
    pub(super) commands: Vec<FrameCommand>,
}

impl DrawList {
    /// Creates an empty draw list.
    ///
    /// Draw lists are standalone command buffers: they can be built before a frame is acquired
    /// and submitted later with [`DrawListRenderer::submit_draw_list`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues an update for a previously created uniform.
    pub fn update_uniform<T: AsUniformBuffer>(&mut self, uniform: UniformId, data: &T) {
        let encoded = match data.encode_bytes() {
            Ok(encoded) => encoded,
            Err(error) => {
                tracing::warn!("Could not encode queued uniform update for {uniform:?}: {error}");
                return;
            }
        };
        self.commands
            .push(FrameCommand::UpdateUniform(UpdateUniform {
                uniform,
                data: encoded,
            }));
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
            .push(FrameCommand::UpdateTextureRegion(UpdateTextureRegion {
                texture,
                origin,
                size,
                data: data.to_vec(),
            }));
    }

    /// Queues a resize of a render target. Executes before any draw commands
    /// in the same draw list, so subsequent draws see the new size immediately.
    pub fn resize_render_target(&mut self, render_target: RenderTargetId, size: UVec2) {
        self.commands
            .push(FrameCommand::ResizeRenderTarget(ResizeRenderTarget {
                render_target,
                size,
            }));
    }

    /// Queues an indexed draw using the provided mesh and material.
    pub fn draw_mesh(&mut self, render_target: RenderTarget, mesh: MeshId, material: MaterialId) {
        self.commands.push(FrameCommand::DrawMesh(DrawMesh {
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

        self.commands.push(FrameCommand::Draw(Draw {
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
            .push(FrameCommand::DrawMeshInstanced(DrawMeshInstanced {
                render_target,
                mesh,
                material,
                instance_buffer_layout: I::layout(),
                instance_data: match I::encode_slice(instances) {
                    Ok(encoded) => encoded,
                    Err(error) => {
                        tracing::warn!(
                            "Could not encode instance buffer for draw on {mesh:?}: {error}"
                        );
                        return;
                    }
                },
                instance_count: instances.len() as u32,
            }));
    }
}
