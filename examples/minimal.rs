use granite::prelude::*;

const SHADER: &str = r"
struct VertexOut {
    @builtin(position) ndc: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex fn vertex(@builtin(vertex_index) index: u32) -> VertexOut {
    let x = f32(1 - i32(index)) * 0.5;
    let y = f32(i32(index & 1u) * 2 - 1) * 0.5;

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
    pipeline: wgpu::RenderPipeline,
}

impl Minimal {
    fn new(surface: &Surface, renderer: &Renderer) -> Self {
        Self {
            pipeline: Self::create_render_pipeline(surface, renderer),
        }
    }

    fn create_render_pipeline(surface: &Surface, renderer: &Renderer) -> wgpu::RenderPipeline {
        let module = renderer
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(SHADER)),
            });

        renderer
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: None,
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
}

impl Scene for Minimal {
    fn render(&mut self, _surface: &Surface, view: &mut Frame) {
        let mut render_pass = view.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view.view,
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
    struct NewMinimal;
    impl NewScene for NewMinimal {
        type Target = Minimal;

        fn new(&self, surface: &Surface, renderer: &Renderer) -> Self::Target {
            Minimal::new(surface, renderer)
        }
    }
    granite::run(NewMinimal);
}
