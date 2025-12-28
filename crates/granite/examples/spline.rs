use glam::{Mat4, UVec2, Vec2};
use granite::prelude::*;
use wgpu::util::DeviceExt;

struct Spline {
    window_size: Option<UVec2>,
    uniforms_buffer: wgpu::Buffer,
    uniforms_bind_group: wgpu::BindGroup,

    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
}

impl Spline {
    fn new(renderer: &RenderContext, surface_config: &SurfaceConfig) -> Self {
        let (vertex_buffer, vertex_count) = {
            let points = vec![
                Vec2::new(100.0, 100.0),
                Vec2::new(1000.0, 400.0),
                Vec2::new(300.0, 800.0),
                Vec2::new(900.0, 1100.0),
            ];

            // let points = sample_catmull_rom_spline(&points, 32);
            let points = sample_catmull_rom_spline(&points, 20.0);
            let vertices = generate_polyline(&points, 40.0);

            (
                renderer
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("spline_vertices"),
                        contents: bytemuck::cast_slice(&vertices),
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    }),
                vertices.len() as u32,
            )
        };

        let projection = Mat4::orthographic_rh(
            0.0,
            surface_config.width as f32,
            surface_config.height as f32,
            0.0,
            0.0,
            1.0,
        );

        let uniforms_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("uniforms_buffer"),
                    contents: bytemuck::cast_slice(&[projection]),
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
                });

        let uniforms_bind_group_layout =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("uniforms_bind_group_layout"),
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

        let uniforms_bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("uniforms_bind_group"),
                layout: &uniforms_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniforms_buffer.as_entire_binding(),
                }],
            });

        let render_pipeline = {
            let module = renderer
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("spline_shader_module"),
                    source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                        r"
                        @group(0) @binding(0) var<uniform> projection: mat4x4<f32>;

                        @vertex fn vertex(@location(0) position: vec2<f32>) -> @builtin(position) vec4<f32> {
                            return projection * vec4<f32>(position, 0.0, 1.0);
                        }

                        @fragment fn fragment() -> @location(0) vec4<f32> {
                            let base_color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
                            return base_color;
                        }
                        ",
                    )),
                });

            let layout = renderer
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("spline_pipeline_layout"),
                    bind_group_layouts: &[&uniforms_bind_group_layout],
                    immediate_size: 0,
                });

            renderer
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("spline_render_pipeline"),
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: None,
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[wgpu::VertexBufferLayout {
                            array_stride: std::mem::size_of::<Vec2>() as wgpu::BufferAddress,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                        }],
                    },
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        ..Default::default()
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: None,
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: surface_config.format,
                            blend: Some(wgpu::BlendState {
                                color: wgpu::BlendComponent {
                                    src_factor: wgpu::BlendFactor::SrcAlpha,
                                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                    operation: wgpu::BlendOperation::Add,
                                },
                                alpha: wgpu::BlendComponent {
                                    src_factor: wgpu::BlendFactor::One,
                                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                    operation: wgpu::BlendOperation::Add,
                                },
                            }),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview_mask: None,
                    cache: None,
                })
        };

        Self {
            window_size: None,
            uniforms_buffer,
            uniforms_bind_group,

            render_pipeline,
            vertex_buffer,
            vertex_count,
        }
    }

    fn calculate_projection(window_size: UVec2) -> Mat4 {
        Mat4::orthographic_rh(
            0.0,
            window_size.x as f32,
            window_size.y as f32,
            0.0,
            0.0,
            1.0,
        )
    }
}

impl Scene for Spline {
    fn event(&mut self, event: &SceneEvent) {
        match event {
            SceneEvent::WindowResized { width, height } => {
                self.window_size = Some(UVec2::new(*width, *height));
            }
        }
    }

    fn render(
        &mut self,
        renderer: &RenderContext,
        surface: &Surface,
    ) -> impl Iterator<Item = wgpu::CommandBuffer> {
        if let Some(window_size) = self.window_size.take() {
            let projection = Self::calculate_projection(window_size);
            renderer.queue.write_buffer(
                &self.uniforms_buffer,
                0,
                bytemuck::cast_slice(&[projection]),
            );
        }

        let mut encoder = renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("spline_command_encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("spline_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_bind_group(0, &self.uniforms_bind_group, &[]);
            render_pass.draw(0..self.vertex_count, 0..1);
        }

        std::iter::once(encoder.finish())
    }
}

fn main() -> Result<(), winit::error::EventLoopError> {
    granite::run(Spline::new)
}

fn sample_catmull_rom_spline(points: &[Vec2], quality: f32) -> Vec<Vec2> {
    fn catmull_rom(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
        let t2 = t * t;
        let t3 = t2 * t;

        0.5 * ((2.0 * p1)
            + (-p0 + p2) * t
            + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
            + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
    }

    fn calculate_segment_count(p0: Vec2, p1: Vec2, quality: f32, min: usize, max: usize) -> usize {
        let dist = p0.distance(p1);
        let segments = (dist * quality).ceil() as usize;
        segments.clamp(min, max)
    }

    let mut result = Vec::new();

    // Duplicate endpoints
    let mut extended = vec![points[0]];
    extended.extend_from_slice(points);
    extended.push(*points.last().unwrap());

    for window in extended.windows(4) {
        let mid0 = window[1];
        let mid1 = window[2];

        let seg_count = calculate_segment_count(mid0, mid1, quality, 4, 32);
        dbg!(seg_count);

        for i in 0..seg_count {
            let t = i as f32 / seg_count as f32;
            result.push(catmull_rom(window[0], window[1], window[2], window[3], t));
        }
    }

    result.push(*points.last().unwrap());

    result
}

fn generate_polyline(points: &[Vec2], thickness: f32) -> Vec<Vec2> {
    let half_thick = thickness / 2.0;
    let mut vertices = Vec::new();

    let len = points.len();

    // Compute normals for each segment first
    let mut normals = Vec::new();
    for i in 0..len - 1 {
        let dir = (points[i + 1] - points[i]).normalize();
        normals.push(Vec2::new(-dir.y, dir.x));
    }

    for i in 0..len {
        let p = points[i];

        // Compute miter at current vertex
        let miter = if i == 0 {
            // First point (use first normal)
            normals[0]
        } else if i == len - 1 {
            // Last point (use last normal)
            normals[len - 2]
        } else {
            // Average normals at the joint
            let n0 = normals[i - 1];
            let n1 = normals[i];
            (n0 + n1).normalize()
        };

        // Compute miter length to ensure corners touch perfectly
        let miter_length = if i == 0 || i == len - 1 {
            half_thick
        } else {
            // At joints, scale by inverse dot product to ensure corners match exactly
            let dot = miter.dot(normals[i]);
            half_thick / dot
        };

        let offset = miter * miter_length;

        // Store both sides of the offset points
        vertices.push(p + offset); // "left" side
        vertices.push(p - offset); // "right" side
    }

    // Generate triangles (two triangles per segment quad)
    let mut final_vertices = Vec::new();
    for i in 0..len - 1 {
        let idx = i * 2;

        let v0 = vertices[idx]; // current left
        let v1 = vertices[idx + 2]; // next left
        let v2 = vertices[idx + 3]; // next right
        let v3 = vertices[idx + 1]; // current right

        final_vertices.extend_from_slice(&[
            v0, v1, v2, // triangle 1
            v2, v3, v0, // triangle 2
        ]);
    }

    final_vertices
}
