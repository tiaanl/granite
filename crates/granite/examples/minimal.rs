use glam::{Vec2, Vec4};
use granite::macros::{instance_buffer, vertex_buffer};
use granite::prelude::*;

const SHADER: &str = r"
struct VertexIn {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex fn vertex(vertex: VertexIn) -> VertexOut {
    let position = vec4<f32>(vertex.position.xy, 0.0, 1.0);
    let color = vertex.color;

    return VertexOut(position, color);
}

@fragment fn fragment(vertex: VertexOut) -> @location(0) vec4<f32> {
    return vertex.color;
}
";

struct MinimalBuilder;

struct Minimal {
    mesh: MeshId,
    material: MaterialId,
}

#[vertex_buffer]
struct Vertex {
    position: Vec4,
    color: Vec4,
}

#[instance_buffer]
struct Instance {
    position: Vec2,
}

impl SceneBuilder for MinimalBuilder {
    type Target = Minimal;

    fn build(&self, renderer: &mut Renderer) -> Self::Target {
        let vertices = &[
            Vertex {
                position: Vec4::new(-0.5, -0.5, 0.0, 0.0),
                color: Vec4::new(1.0, 0.0, 0.0, 1.0),
            },
            Vertex {
                position: Vec4::new(0.0, 0.5, 0.0, 0.0),
                color: Vec4::new(0.0, 1.0, 0.0, 1.0),
            },
            Vertex {
                position: Vec4::new(0.5, -0.5, 0.0, 0.0),
                color: Vec4::new(0.0, 0.0, 1.0, 1.0),
            },
        ];

        let mesh = renderer.create_mesh("triangle", vertices, &[0, 1, 2]);

        let shader = renderer.create_shader("minimal", SHADER);
        let vertex_shader = renderer.create_vertex_shader(shader, "vertex");
        let fragment_shader = renderer.create_fragment_shader(shader, "fragment");
        let material = renderer
            .create_material(vertex_shader, fragment_shader)
            .build();

        Self::Target { mesh, material }
    }
}

impl Scene for Minimal {
    fn render(&mut self, frame: &mut Frame) {
        let instances = [Instance {
            position: Vec2::ZERO,
        }];
        frame.draw_mesh_instanced(RenderTarget::Surface, self.mesh, self.material, &instances);
    }
}

fn main() {
    granite::run(MinimalBuilder).unwrap();
}
