use crate::{depth_buffer::DepthBufferRecord, draw_list::RenderTarget};

use super::*;

pub struct PreparedDraw {
    pub key: RenderPipelineKey,
    pub bind_groups_to_set: Vec<ResolvedDrawBindGroup>,
    pub depth_state: Option<MaterialDepthState>,
}

impl PreparedDraw {
    pub fn try_new(
        renderer: &mut DrawListRenderer,
        surface_format: wgpu::TextureFormat,
        render_target: RenderTarget,
        mesh: Option<MeshId>,
        material: MaterialId,
        instance_buffer_layout: Option<VertexBufferLayout>,
    ) -> Option<PreparedDraw> {
        let (vertex_shader, fragment_shader, draw_bindings, blend_mode, depth_state) = {
            let Some(material) = renderer.materials.get(material) else {
                tracing::warn!("Invalid material id ({:?})", material);
                return None;
            };
            (
                material.vertex_shader,
                material.fragment_shader,
                material.bindings.clone(),
                material.blend_mode,
                material.depth_state,
            )
        };

        let vertex_buffer_layout = if let Some(mesh_id) = mesh {
            let Some(vertex_buffer_layout_id) = renderer
                .meshes
                .get(mesh_id)
                .map(|mesh| mesh.vertex_buffer_layout_id)
            else {
                tracing::warn!("Invalid mesh id ({:?})", mesh_id);
                return None;
            };
            Some(vertex_buffer_layout_id)
        } else {
            None
        };

        let instance_buffer_layout = instance_buffer_layout
            .map(|layout| renderer.get_or_create_instance_buffer_layout(layout));

        let resolved_bindings = renderer.resolve_draw_bindings(draw_bindings.as_slice())?;
        let Some(pipeline_layout_id) =
            renderer.get_or_create_pipeline_layout(resolved_bindings.pipeline_layout_key)
        else {
            tracing::warn!("Could not ensure a valid pipeline layout!");
            return None;
        };

        let depth_stencil = depth_state.map(|depth_state| RenderPipelineDepthKey {
            format: DepthBufferRecord::FORMAT,
            compare: depth_state.compare,
            write_enabled: depth_state.write_enabled,
        });

        let key = RenderPipelineKey {
            render_target_format: renderer.render_target_format(surface_format, render_target)?,
            depth_stencil,
            vertex_buffer_layout,
            instance_buffer_layout,
            pipeline_layout: pipeline_layout_id,
            vertex_shader,
            fragment_shader,
            blend_mode,
        };

        if !renderer.ensure_render_pipeline(key) {
            tracing::warn!("Could not ensure a valid render pipeline!");
            return None;
        }

        Some(PreparedDraw {
            key,
            bind_groups_to_set: resolved_bindings.bind_groups_to_set,
            depth_state,
        })
    }
}
