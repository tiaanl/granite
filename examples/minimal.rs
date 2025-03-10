use granite::{glam::*, prelude::*};
use wgpu::util::DeviceExt;

const SHADER: &str = r"
@group(0) @binding(0) var<uniform> scale: vec4<f32>;

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex fn vertex(@builtin(vertex_index) index: u32) -> VertexOut {
    let x = f32(1 - i32(index)) * 0.5 * scale.x;
    let y = f32(i32(index & 1u) * 2 - 1) * 0.5 * scale.x;

    let r = f32(index == 0u);
    let g = f32(index == 1u);
    let b = f32(index == 2u);

    return VertexOut(
        vec4(x, y, 0.0, 1.0),
        vec4(r, g, b, 1.0),
    );
}

@fragment fn fragment(vertex: VertexOut) -> @location(0) vec4<f32> {
    return vertex.color;
}
";

struct Minimal {
    /// The render pipeline used for the triangle.
    pipeline: wgpu::RenderPipeline,

    /// Buffer holding the uniform data.
    uniforms_buffer: wgpu::Buffer,

    /// The bind group used to reference the uniforms buffer.
    uniforms_bind_group: wgpu::BindGroup,

    /// A dynamic scale value applied to the triangle.
    scale: f32,
}

impl Minimal {
    fn new(surface: &Surface, renderer: &Renderer) -> Self {
        let scale = 1.0;

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

        let uniforms_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("uniforms_bind_group"),
                    contents: bytemuck::cast_slice(&[Vec4::new(scale, 0.0, 0.0, 0.0)]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
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

        let pipeline = Self::create_render_pipeline(surface, renderer, &uniforms_bind_group_layout);

        Self {
            pipeline,
            uniforms_buffer,
            uniforms_bind_group,
            scale,
        }
    }

    fn create_render_pipeline(
        surface: &Surface,
        renderer: &Renderer,
        uniforms_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        let module = renderer
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SHADER)),
            });

        let layout = renderer
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[uniforms_bind_group_layout],
                push_constant_ranges: &[],
            });

        renderer
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(surface.format.into())],
                }),
                multiview: None,
                cache: None,
            })
    }

    fn upload_uniform(&mut self, renderer: &Renderer) {
        let data = Vec4::new(self.scale, 0.0, 0.0, 1.0);
        renderer
            .queue
            .write_buffer(&self.uniforms_buffer, 0, bytemuck::cast_slice(&[data]));
    }
}

impl Scene for Minimal {
    fn update(&mut self, input: &InputState, _time_delta: f32) {
        if input.key_pressed(KeyCode::KeyW) {
            self.scale = (self.scale - 0.1).max(0.1);
        }
        if input.key_pressed(KeyCode::KeyS) {
            self.scale = (self.scale + 0.1).min(2.0);
        }
    }

    fn render(&mut self, _surface: &Surface, frame: &mut Frame) {
        self.upload_uniform(frame.renderer);

        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations::default(),
                })],
                ..Default::default()
            });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.uniforms_bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

fn main() -> Result<(), winit::error::EventLoopError> {
    granite::run(Minimal::new)
}
