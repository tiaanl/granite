use granite_wgpu::{glam::*, prelude::*};

const SHADER: &str = r"
struct Camera {
    proj: mat4x4<f32>,
    view: mat4x4<f32>,
}
@group(0) @binding(0) var<uniform> u_camera: Camera;

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
        u_camera.proj * u_camera.view * vec4(x, y, 0.0, 1.0),
        vec4(r, g, b, 1.0),
    );
}

@fragment fn fragment(vertex: VertexOut) -> @location(0) vec4<f32> {
    return vertex.color;
}
";

struct Minimal {
    camera: Camera,
    gpu_camera: GpuCamera,
    pipeline: wgpu::RenderPipeline,
}

impl Minimal {
    fn new(surface: &Surface, renderer: &Renderer) -> Self {
        let camera = Camera::new(Vec3::new(0.0, 0.0, 0.5), Quat::IDENTITY);
        let gpu_camera = GpuCamera::new(&renderer.device);
        let pipeline =
            Self::create_render_pipeline(surface, renderer, &gpu_camera.bind_group_layout);

        Self {
            camera,
            gpu_camera,
            pipeline,
        }
    }

    fn create_render_pipeline(
        surface: &Surface,
        renderer: &Renderer,
        camera_layout: &wgpu::BindGroupLayout,
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
                bind_group_layouts: &[camera_layout],
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
}

impl Scene for Minimal {
    fn update(&mut self, input: &InputState, time_delta: f32) {
        if input.key_pressed(KeyCode::KeyW) {
            self.camera.move_forward(-time_delta * 0.1);
        }
        if input.key_pressed(KeyCode::KeyS) {
            self.camera.move_forward(time_delta * 0.1);
        }
    }

    fn render(&mut self, _surface: &Surface, frame: &mut Frame) {
        self.gpu_camera
            .upload(&frame.renderer.queue, &self.camera.calculate_matrices());

        let mut render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

        render_pass.set_bind_group(0, &self.gpu_camera.bind_group, &[]);
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
    granite_wgpu::run(NewMinimal);
}
