use granite::renderer::Frame;

use crate::draw_list::{DrawList, RenderTarget};

use super::*;

impl DrawListRenderer {
    /// Begins recording a new higher-level draw list.
    ///
    /// This is a convenience wrapper around [`DrawList::new`].
    pub fn create_draw_list(&self) -> DrawList {
        DrawList::new()
    }

    /// Executes all commands in a draw list into the provided frame.
    pub fn submit_draw_list(&mut self, frame: &mut Frame, draw_list: DrawList) {
        let DrawList { commands } = draw_list;
        let mut frame_instance_buffers: Vec<wgpu::Buffer> = Vec::new();

        for command in commands.iter() {
            match command {
                commands::FrameCommand::UpdateUniform(command) => command.execute(self),
                commands::FrameCommand::UpdateTextureRegion(command) => command.execute(self),
                commands::FrameCommand::ResizeRenderTarget(command) => command.execute(self),
                commands::FrameCommand::Draw(command) => {
                    self.ensure_render_target_ready(frame, command.render_target);
                    if let Some(mut pass) = self.create_render_pass_for_render_target(
                        &mut frame.encoder,
                        &frame.view,
                        command.render_target,
                    ) {
                        command.execute(self, frame.surface_format, &mut pass);
                    }
                }
                commands::FrameCommand::DrawMesh(command) => {
                    self.ensure_render_target_ready(frame, command.render_target);

                    if let Some(mut pass) = self.create_render_pass_for_render_target(
                        &mut frame.encoder,
                        &frame.view,
                        command.render_target,
                    ) {
                        command.execute(self, frame.surface_format, &mut pass);
                    }
                }
                commands::FrameCommand::DrawMeshInstanced(command) => {
                    self.ensure_render_target_ready(frame, command.render_target);
                    if let Some(mut pass) = self.create_render_pass_for_render_target(
                        &mut frame.encoder,
                        &frame.view,
                        command.render_target,
                    ) {
                        command.execute(
                            self,
                            frame.surface_format,
                            &mut pass,
                            &mut frame_instance_buffers,
                        );
                    }
                }
            }
        }
    }

    fn create_render_pass_for_render_target<'encoder>(
        &self,
        encoder: &'encoder mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
        render_target: RenderTarget,
    ) -> Option<wgpu::RenderPass<'encoder>> {
        let view = match render_target {
            RenderTarget::Surface => surface_view,
            RenderTarget::Custom(id) => {
                let record = self.render_targets.get(id)?;
                record.view.as_ref()?
            }
        };

        Some(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("main_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        }))
    }

    pub(super) fn get_or_create_vertex_buffer_layout(&mut self, layout: VertexBufferLayout) -> Id {
        self.vertex_buffer_layouts.get_or_insert(layout)
    }

    pub(super) fn get_or_create_instance_buffer_layout(
        &mut self,
        layout: VertexBufferLayout,
    ) -> Id {
        self.instance_buffer_layouts.get_or_insert(layout)
    }

    pub(super) fn resolve_draw_bindings(
        &mut self,
        draw_bindings: &[bindings::DrawBinding],
    ) -> Option<ResolvedDrawBindings> {
        if draw_bindings.is_empty() {
            return Some(ResolvedDrawBindings {
                bind_groups_to_set: Vec::new(),
                pipeline_layout_key: PipelineLayoutKey {
                    bind_group_layouts: Vec::new(),
                },
            });
        }

        let mut draw_bindings = draw_bindings.to_vec();
        draw_bindings.sort_by_key(|binding| (binding.group, binding.binding));

        for pair in draw_bindings.windows(2) {
            if pair[0].group == pair[1].group && pair[0].binding == pair[1].binding {
                tracing::warn!(
                    "Duplicate draw binding for @group({}) @binding({})",
                    pair[0].group,
                    pair[0].binding
                );
                return None;
            }
        }

        let mut grouped_bindings: Vec<(
            u32,
            Vec<BindGroupBindingKey>,
            Vec<BindGroupLayoutBindingKey>,
        )> = Vec::new();

        for draw_binding in draw_bindings.into_iter() {
            let (bind_group_binding_key, bind_group_layout_binding_key) =
                match draw_binding.resource {
                    bindings::DrawBindingResource::Uniform(uniform_binding_id) => {
                        let Some(uniform) = self.uniforms.get(uniform_binding_id) else {
                            tracing::warn!("Invalid uniform id ({uniform_binding_id:?})");
                            return None;
                        };

                        (
                            BindGroupBindingKey {
                                binding: draw_binding.binding,
                                resource: BindGroupBindingResourceKey::Uniform(uniform_binding_id),
                            },
                            BindGroupLayoutBindingKey {
                                binding: draw_binding.binding,
                                visibility: uniform.visibility,
                                ty: BindGroupLayoutBindingTypeKey::Uniform,
                                min_binding_size: Some(uniform.min_binding_size),
                            },
                        )
                    }
                    bindings::DrawBindingResource::Texture {
                        texture,
                        visibility,
                    } => {
                        if self.textures.get(texture).is_none() {
                            tracing::warn!("Invalid texture id ({texture:?})");
                            return None;
                        }

                        (
                            BindGroupBindingKey {
                                binding: draw_binding.binding,
                                resource: BindGroupBindingResourceKey::Texture(texture),
                            },
                            BindGroupLayoutBindingKey {
                                binding: draw_binding.binding,
                                visibility,
                                ty: BindGroupLayoutBindingTypeKey::Texture,
                                min_binding_size: None,
                            },
                        )
                    }
                    bindings::DrawBindingResource::RenderTarget {
                        render_target,
                        visibility,
                    } => {
                        if self.render_targets.get(render_target).is_none() {
                            tracing::warn!("Invalid render target id ({render_target:?})");
                            return None;
                        }

                        (
                            BindGroupBindingKey {
                                binding: draw_binding.binding,
                                resource: BindGroupBindingResourceKey::RenderTarget(render_target),
                            },
                            BindGroupLayoutBindingKey {
                                binding: draw_binding.binding,
                                visibility,
                                ty: BindGroupLayoutBindingTypeKey::Texture,
                                min_binding_size: None,
                            },
                        )
                    }
                    bindings::DrawBindingResource::Sampler {
                        sampler,
                        visibility,
                    } => {
                        if self.samplers.get(sampler).is_none() {
                            tracing::warn!("Invalid sampler id ({sampler:?})");
                            return None;
                        }

                        (
                            BindGroupBindingKey {
                                binding: draw_binding.binding,
                                resource: BindGroupBindingResourceKey::Sampler(sampler),
                            },
                            BindGroupLayoutBindingKey {
                                binding: draw_binding.binding,
                                visibility,
                                ty: BindGroupLayoutBindingTypeKey::Sampler,
                                min_binding_size: None,
                            },
                        )
                    }
                };

            if let Some((group, bindings, layout_bindings)) = grouped_bindings.last_mut()
                && *group == draw_binding.group
            {
                bindings.push(bind_group_binding_key);
                layout_bindings.push(bind_group_layout_binding_key);
                continue;
            }

            grouped_bindings.push((
                draw_binding.group,
                vec![bind_group_binding_key],
                vec![bind_group_layout_binding_key],
            ));
        }

        let mut bind_groups_to_set = Vec::with_capacity(grouped_bindings.len());
        for (group, bindings, layout_bindings) in grouped_bindings.into_iter() {
            let bind_group_layout =
                self.get_or_create_bind_group_layout_for_key(BindGroupLayoutKey {
                    bindings: layout_bindings,
                })?;
            let bind_group = self.get_or_create_bind_group_for_key(BindGroupKey {
                bind_group_layout,
                bindings,
            })?;

            bind_groups_to_set.push(ResolvedDrawBindGroup {
                slot: group,
                bind_group,
                bind_group_layout,
            });
        }

        let max_group = bind_groups_to_set
            .iter()
            .map(|bind_group| bind_group.slot)
            .max()
            .unwrap_or(0);
        let empty_bind_group_layout = self.get_or_create_empty_bind_group_layout();
        let mut bind_group_layouts = vec![empty_bind_group_layout; max_group as usize + 1];
        for bind_group in bind_groups_to_set.iter() {
            bind_group_layouts[bind_group.slot as usize] = bind_group.bind_group_layout;
        }

        Some(ResolvedDrawBindings {
            bind_groups_to_set,
            pipeline_layout_key: PipelineLayoutKey { bind_group_layouts },
        })
    }

    fn get_or_create_empty_bind_group_layout(&mut self) -> Id {
        if let Some(bind_group_layout) = self.empty_bind_group_layout {
            return bind_group_layout;
        }

        let bind_group_layout = self.create_bind_group_layout("empty_bind_group_layout", &[]);
        self.empty_bind_group_layout = Some(bind_group_layout);
        bind_group_layout
    }

    fn get_or_create_bind_group_layout_for_key(&mut self, key: BindGroupLayoutKey) -> Option<Id> {
        if let Some(bind_group_layout) = self.bind_group_layouts.get_id(&key) {
            return Some(bind_group_layout);
        }

        let mut entries = Vec::with_capacity(key.bindings.len());
        for binding in key.bindings.iter() {
            let ty = match binding.ty {
                BindGroupLayoutBindingTypeKey::Uniform => wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: binding.min_binding_size,
                },
                BindGroupLayoutBindingTypeKey::Texture => wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                BindGroupLayoutBindingTypeKey::Sampler => {
                    wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)
                }
            };

            entries.push(wgpu::BindGroupLayoutEntry {
                binding: binding.binding,
                visibility: binding.visibility.as_wgpu(),
                ty,
                count: None,
            });
        }

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: entries.as_slice(),
                });
        let bind_group_layout_id = self.bind_group_layouts.insert_keyed(key, bind_group_layout);
        Some(bind_group_layout_id)
    }

    fn get_or_create_bind_group_for_key(&mut self, key: BindGroupKey) -> Option<Id> {
        if let Some(bind_group) = self.bind_groups.get_id(&key) {
            return Some(bind_group);
        }

        let bind_group_layout = self.bind_group_layouts.get(key.bind_group_layout)?;
        let bind_group = {
            let mut entries = Vec::with_capacity(key.bindings.len());
            for binding in key.bindings.iter() {
                let resource = match binding.resource {
                    BindGroupBindingResourceKey::Uniform(uniform_binding_id) => {
                        let uniform = self.uniforms.get(uniform_binding_id)?;
                        let buffer = self.buffers.get(uniform.buffer)?;
                        buffer.as_entire_binding()
                    }
                    BindGroupBindingResourceKey::Texture(texture_id) => {
                        let texture = self.textures.get(texture_id)?;
                        wgpu::BindingResource::TextureView(&texture.view)
                    }
                    BindGroupBindingResourceKey::RenderTarget(render_target_id) => {
                        let render_target = self.render_targets.get(render_target_id)?;
                        wgpu::BindingResource::TextureView(render_target.view.as_ref()?)
                    }
                    BindGroupBindingResourceKey::Sampler(sampler_id) => {
                        let sampler = self.samplers.get(sampler_id)?;
                        wgpu::BindingResource::Sampler(sampler)
                    }
                };

                entries.push(wgpu::BindGroupEntry {
                    binding: binding.binding,
                    resource,
                });
            }

            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("draw_bind_group"),
                layout: bind_group_layout,
                entries: entries.as_slice(),
            })
        };

        let bind_group_id = self
            .bind_groups
            .insert_keyed(key, BindGroupRecord { bind_group });
        Some(bind_group_id)
    }

    pub(super) fn get_or_create_pipeline_layout(&mut self, key: PipelineLayoutKey) -> Option<Id> {
        if let Some(pipeline_layout_id) = self.pipeline_layouts.get_id(&key) {
            return Some(pipeline_layout_id);
        }

        let mut bind_group_layouts = Vec::with_capacity(key.bind_group_layouts.len());
        for bind_group_layout_id in key.bind_group_layouts.iter() {
            let bind_group_layout = self.bind_group_layouts.get(*bind_group_layout_id)?;
            bind_group_layouts.push(bind_group_layout);
        }

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: bind_group_layouts.as_slice(),
                immediate_size: 0,
            });

        let pipeline_layout_id = self.pipeline_layouts.insert_keyed(key, pipeline_layout);
        Some(pipeline_layout_id)
    }

    pub(super) fn ensure_render_pipeline(&mut self, key: RenderPipelineKey) -> bool {
        if !self.render_pipeline_cache.contains_key(&key) {
            let Some(render_pipeline) = self.create_render_pipeline(key) else {
                return false;
            };
            self.render_pipeline_cache.insert(key, render_pipeline);
        }
        true
    }

    fn create_render_pipeline(&self, key: RenderPipelineKey) -> Option<wgpu::RenderPipeline> {
        tracing::debug!("Creating render pipeline for {key:?}");

        let device = &self.device;
        let pipeline_layout = self.pipeline_layouts.get(key.pipeline_layout)?;

        let vertex_buffer_layout = key
            .vertex_buffer_layout
            .and_then(|vertex_buffer_layout| self.vertex_buffer_layouts.get(vertex_buffer_layout));
        if key.vertex_buffer_layout.is_some() && vertex_buffer_layout.is_none() {
            tracing::warn!("Vertex buffer layout not found");
            return None;
        }
        let vertex_attributes = vertex_buffer_layout
            .map(|vertex_buffer_layout| mesh::vertex_attributes(vertex_buffer_layout, 0));

        let instance_buffer_layout = {
            key.instance_buffer_layout
                .and_then(|instance_buffer_layout| {
                    if let Some(instance_buffer_layout) =
                        self.instance_buffer_layouts.get(instance_buffer_layout)
                    {
                        Some(instance_buffer_layout)
                    } else {
                        tracing::warn!("Instance buffer layout not found");
                        None
                    }
                })
        };

        let instance_attribute_start = vertex_attributes
            .as_ref()
            .map_or(0, |vertex_attributes| vertex_attributes.len() as u32);
        let instance_attributes = instance_buffer_layout.map(|instance_buffer_layout| {
            mesh::vertex_attributes(instance_buffer_layout, instance_attribute_start)
        });

        let vertex_shader = self.vertex_shaders.get(key.vertex_shader)?;
        let fragment_shader = self.fragment_shaders.get(key.fragment_shader)?;
        let vertex_shader_module = self.shaders.get(vertex_shader.shader_module)?;
        let fragment_shader_module = self.shaders.get(fragment_shader.shader_module)?;

        let blend = match key.blend_mode {
            BlendMode::Opaque => None,
            BlendMode::AlphaBlend => Some(wgpu::BlendState::ALPHA_BLENDING),
            BlendMode::Additive => Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
            }),
            BlendMode::Premultiplied => Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
        };
        let targets = &[Some(wgpu::ColorTargetState {
            format: key.render_target_format,
            blend,
            write_mask: wgpu::ColorWrites::ALL,
        })];

        let mut buffers: Vec<wgpu::VertexBufferLayout<'_>> = Vec::new();
        if let (Some(vertex_buffer_layout), Some(vertex_attributes)) =
            (&vertex_buffer_layout, &vertex_attributes)
        {
            buffers.push(wgpu::VertexBufferLayout {
                array_stride: vertex_buffer_layout.size,
                step_mode: vertex_buffer_layout.step_mode,
                attributes: vertex_attributes.as_slice(),
            });
        }
        if let (Some(instance_buffer_layout), Some(instance_attributes)) =
            (&instance_buffer_layout, &instance_attributes)
        {
            buffers.push(wgpu::VertexBufferLayout {
                array_stride: instance_buffer_layout.size,
                step_mode: instance_buffer_layout.step_mode,
                attributes: instance_attributes.as_slice(),
            });
        }

        Some(
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &vertex_shader_module.shader_module,
                    entry_point: vertex_shader.entry_point.as_deref(),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: buffers.as_slice(),
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &fragment_shader_module.shader_module,
                    entry_point: fragment_shader.entry_point.as_deref(),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets,
                }),
                multiview_mask: None,
                cache: None,
            }),
        )
    }
}
