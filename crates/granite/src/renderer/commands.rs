use super::*;
use wgpu::util::DeviceExt;

pub(super) struct DrawIndexedCommand {
    pub mesh: MeshId,
    pub material: MaterialId,
    pub instance_buffer_layout: VertexBufferLayout,
    pub instance_data: Vec<u8>,
    pub instance_count: u32,
}

pub(super) struct UniformUpdateCommand {
    pub uniform: UniformId,
    pub data: Vec<u8>,
}

pub(super) enum FrameCommand {
    UpdateUniform(UniformUpdateCommand),
    DrawIndexed(DrawIndexedCommand),
}

impl UniformUpdateCommand {
    pub(super) fn execute(&self, renderer: &mut Renderer) {
        let _ = renderer.write_uniform_bytes(self.uniform, self.data.as_slice());
    }
}

impl DrawIndexedCommand {
    pub(super) fn execute(
        &self,
        renderer: &mut Renderer,
        render_pass: &mut wgpu::RenderPass<'_>,
        frame_instance_buffers: &mut Vec<wgpu::Buffer>,
    ) {
        if self.instance_count == 0 || self.instance_data.is_empty() {
            return;
        }

        let (vertex_shader, fragment_shader, draw_bindings) = {
            let Some(material) = renderer.materials.get(self.material) else {
                tracing::warn!("Invalid material id ({:?})", self.material);
                return;
            };
            (
                material.vertex_shader,
                material.fragment_shader,
                material.bindings.clone(),
            )
        };

        let Some(vertex_buffer_layout_id) = renderer
            .meshes
            .get(self.mesh)
            .map(|mesh| mesh.vertex_buffer_layout_id)
        else {
            tracing::warn!("Invalid mesh id ({:?})", self.mesh);
            return;
        };

        let instance_buffer_layout_id =
            renderer.get_or_create_instance_buffer_layout(self.instance_buffer_layout.clone());

        let Some(resolved_bindings) = renderer.resolve_draw_bindings(draw_bindings.as_slice())
        else {
            return;
        };
        let Some(pipeline_layout_id) =
            renderer.get_or_create_pipeline_layout(resolved_bindings.pipeline_layout_key)
        else {
            tracing::warn!("Could not ensure a valid pipeline layout!");
            return;
        };

        let key = RenderPipelineKey {
            vertex_buffer_layout: vertex_buffer_layout_id,
            instance_buffer_layout: instance_buffer_layout_id,
            pipeline_layout: pipeline_layout_id,
            vertex_shader,
            fragment_shader,
        };

        if !renderer.ensure_render_pipeline(key) {
            tracing::warn!("Could not ensure a valid render pipeline!");
            return;
        }

        let Some(mesh) = renderer.meshes.get(self.mesh) else {
            tracing::warn!("Invalid mesh id ({:?})", self.mesh);
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

        let render_pipeline = &renderer.render_pipeline_cache[&key];
        render_pass.set_pipeline(render_pipeline);
        for bind_group in resolved_bindings.bind_groups_to_set.iter() {
            let Some(bind_group_record) = renderer.bind_groups.get(bind_group.bind_group) else {
                tracing::warn!("Invalid bind group id ({:?})", bind_group.bind_group);
                return;
            };

            render_pass.set_bind_group(bind_group.slot, &bind_group_record.bind_group, &[]);
        }
        render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
        render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..mesh.index_count, 0, 0..self.instance_count);
    }
}
