use super::{prepared_draw::PreparedDraw, *};
use wgpu::util::DeviceExt;

pub(super) enum FrameCommand {
    UpdateUniform(UpdateUniform),
    UpdateTextureRegion(UpdateTextureRegion),
    ResizeRenderTarget(ResizeRenderTarget),
    Draw(Draw),
    DrawMesh(DrawMesh),
    DrawMeshInstanced(DrawMeshInstanced),
}

pub(super) struct Draw {
    pub render_target: RenderTarget,
    pub material: MaterialId,
    pub vertex_count: u32,
}

impl Draw {
    pub(super) fn execute(&self, renderer: &mut Renderer, render_pass: &mut wgpu::RenderPass<'_>) {
        if self.vertex_count == 0 {
            return;
        }

        let Some(prepared_draw) =
            PreparedDraw::try_new(renderer, self.render_target, None, self.material, None)
        else {
            return;
        };

        if !bind_pipeline_and_groups(renderer, render_pass, &prepared_draw) {
            return;
        }

        render_pass.draw(0..self.vertex_count, 0..1);
    }
}

pub(super) struct DrawMesh {
    pub render_target: RenderTarget,
    pub mesh: MeshId,
    pub material: MaterialId,
}

impl DrawMesh {
    pub(super) fn execute(&self, renderer: &mut Renderer, render_pass: &mut wgpu::RenderPass<'_>) {
        let Some(prepared_draw) = PreparedDraw::try_new(
            renderer,
            self.render_target,
            Some(self.mesh),
            self.material,
            None,
        ) else {
            return;
        };
        let Some(index_count) = bind_draw_state(renderer, render_pass, &prepared_draw, self.mesh)
        else {
            return;
        };

        render_pass.draw_indexed(0..index_count, 0, 0..1);
    }
}

pub(super) struct DrawMeshInstanced {
    pub render_target: RenderTarget,
    pub mesh: MeshId,
    pub material: MaterialId,
    pub instance_buffer_layout: VertexBufferLayout,
    pub instance_data: Vec<u8>,
    pub instance_count: u32,
}

impl DrawMeshInstanced {
    pub(super) fn execute(
        &self,
        renderer: &mut Renderer,
        render_pass: &mut wgpu::RenderPass<'_>,
        frame_instance_buffers: &mut Vec<wgpu::Buffer>,
    ) {
        if self.instance_count == 0 || self.instance_data.is_empty() {
            return;
        }

        let Some(prepared_draw) = PreparedDraw::try_new(
            renderer,
            self.render_target,
            Some(self.mesh),
            self.material,
            Some(self.instance_buffer_layout.clone()),
        ) else {
            return;
        };

        frame_instance_buffers.push(renderer.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("frame_instance_buffer"),
                contents: self.instance_data.as_slice(),
                usage: wgpu::BufferUsages::VERTEX,
            },
        ));
        let instance_buffer = frame_instance_buffers.last().unwrap();

        let Some(index_count) = bind_draw_state(renderer, render_pass, &prepared_draw, self.mesh)
        else {
            return;
        };

        render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
        render_pass.draw_indexed(0..index_count, 0, 0..self.instance_count);
    }
}

pub(super) struct UpdateUniform {
    pub uniform: UniformId,
    pub data: Vec<u8>,
}

impl UpdateUniform {
    pub(super) fn execute(&self, renderer: &mut Renderer) {
        let _ = renderer.write_uniform_bytes(self.uniform, self.data.as_slice());
    }
}

pub(super) struct UpdateTextureRegion {
    pub texture: TextureId,
    pub origin: glam::UVec2,
    pub size: glam::UVec2,
    pub data: Vec<u8>,
}

impl UpdateTextureRegion {
    pub(super) fn execute(&self, renderer: &mut Renderer) {
        let _ = renderer.write_texture_rgba8_region(
            self.texture,
            self.origin,
            self.size,
            self.data.as_slice(),
        );
    }
}

pub(super) struct ResizeRenderTarget {
    pub render_target: RenderTargetId,
    pub size: glam::UVec2,
}

impl ResizeRenderTarget {
    pub(super) fn execute(&self, renderer: &mut Renderer) {
        renderer.resize_render_target(self.render_target, self.size);
    }
}

fn bind_draw_state(
    renderer: &Renderer,
    render_pass: &mut wgpu::RenderPass<'_>,
    prepared_draw: &PreparedDraw,
    mesh_id: MeshId,
) -> Option<u32> {
    if !bind_pipeline_and_groups(renderer, render_pass, prepared_draw) {
        return None;
    }

    let Some(mesh) = renderer.meshes.get(mesh_id) else {
        tracing::warn!("Invalid mesh id ({:?})", mesh_id);
        return None;
    };

    render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
    render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

    Some(mesh.index_count)
}

fn bind_pipeline_and_groups(
    renderer: &Renderer,
    render_pass: &mut wgpu::RenderPass<'_>,
    prepared_draw: &PreparedDraw,
) -> bool {
    let render_pipeline = &renderer.render_pipeline_cache[&prepared_draw.key];
    render_pass.set_pipeline(render_pipeline);
    for bind_group in prepared_draw.bind_groups_to_set.iter() {
        let Some(bind_group_record) = renderer.bind_groups.get(bind_group.bind_group) else {
            tracing::warn!("Invalid bind group id ({:?})", bind_group.bind_group);
            return false;
        };

        render_pass.set_bind_group(bind_group.slot, &bind_group_record.bind_group, &[]);
    }

    true
}
