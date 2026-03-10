use granite::{
    app::SceneBuilder,
    renderer::{Frame, Renderer},
    scene::Scene,
};

const SHADER: &str = r"
struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> VertexOut {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 0.6),
        vec2<f32>(-0.6, -0.6),
        vec2<f32>(0.6, -0.6),
    );

    let colors = array<vec3<f32>, 3>(
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(0.0, 0.0, 1.0),
    );

    var out: VertexOut;
    out.clip_position = vec4<f32>(positions[index], 0.0, 1.0);
    out.color = colors[index];
    return out;
}

@fragment
fn fragment(in: VertexOut) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
";

struct MinimalBuilder;

struct Minimal {
    pipeline: wgpu::RenderPipeline,
}

impl SceneBuilder for MinimalBuilder {
    type Target = Minimal;

    fn build(&self, renderer: &mut Renderer) -> Self::Target {
        let shader = renderer
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("minimal_triangle_shader"),
                source: wgpu::ShaderSource::Wgsl(SHADER.into()),
            });

        let layout = renderer
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("minimal_triangle_layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });

        let pipeline = renderer
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("minimal_triangle_pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: renderer.surface_format(),
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });

        Self::Target { pipeline }
    }
}

impl Scene for Minimal {
    fn render(&mut self, frame: &mut Frame) {
        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("minimal_triangle_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
        render_pass.set_pipeline(&self.pipeline);
        render_pass.draw(0..3, 0..1);
    }
}

fn main() {
    granite::run(MinimalBuilder).unwrap();
}
