use glam::{Mat4, UVec2, Vec2};
use granite::{
    WindowEvent,
    app::SceneBuilder,
    renderer::{Frame, Renderer},
    scene::Scene,
};
use granite_draw::{
    DrawListRenderer, FrameContext, MaterialId, MeshId, UniformId,
    draw_list::{DrawList, RenderTarget},
};
use granite_macros::{instance_buffer, uniform_buffer, vertex_buffer};

const SHADER: &str = r"
struct Uniforms {
    projection: mat4x4<f32>,
}

struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) instance_offset: vec2<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex fn vertex(input: VertexIn) -> @builtin(position) vec4<f32> {
    let world_position = input.position + input.instance_offset;
    return uniforms.projection * vec4<f32>(world_position, 0.0, 1.0);
}

@fragment fn fragment() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
";

struct SplineBuilder;

struct Spline {
    draw_list_renderer: DrawListRenderer,
    mesh: MeshId,
    material: MaterialId,
    projection_uniform: UniformId,
    world_size: Vec2,
    pending_projection: Option<ProjectionUniform>,
}

#[vertex_buffer]
struct Vertex {
    position: Vec2,
}

#[instance_buffer]
struct Instance {
    offset: Vec2,
}

#[uniform_buffer(Vertex)]
struct ProjectionUniform {
    projection: Mat4,
}

impl ProjectionUniform {
    fn from_view(world_size: Vec2, window_size: UVec2) -> Self {
        let view_width = window_size.x.max(1) as f32;
        let view_height = window_size.y.max(1) as f32;
        let view_aspect = view_width / view_height;
        let world_aspect = world_size.x / world_size.y;

        let (projection_width, projection_height) = if view_aspect > world_aspect {
            (world_size.y * view_aspect, world_size.y)
        } else {
            (world_size.x, world_size.x / view_aspect)
        };

        let projection =
            Mat4::orthographic_rh(0.0, projection_width, projection_height, 0.0, 0.0, 1.0);

        Self { projection }
    }
}

impl SceneBuilder for SplineBuilder {
    type Target = Spline;

    fn build(self, renderer: &mut Renderer) -> Self::Target {
        let mut draw_list_renderer =
            DrawListRenderer::new(renderer.device.clone(), renderer.queue.clone());
        let points = vec![
            Vec2::new(100.0, 100.0),
            Vec2::new(1000.0, 400.0),
            Vec2::new(300.0, 800.0),
            Vec2::new(900.0, 1100.0),
        ];

        let points = sample_catmull_rom_spline(&points, 20.0);
        let polyline = generate_polyline(&points, 40.0);

        let vertices: Vec<Vertex> = polyline
            .into_iter()
            .map(|position| Vertex { position })
            .collect();
        let indices: Vec<u32> = (0..vertices.len() as u32).collect();

        let mesh =
            draw_list_renderer.create_mesh("spline", vertices.as_slice(), indices.as_slice());

        let bounds = vertices
            .iter()
            .map(|vertex| vertex.position)
            .fold(Vec2::ZERO, |acc, position| acc.max(position));
        let world_size = Vec2::new(bounds.x.ceil() + 64.0, bounds.y.ceil() + 64.0);
        let initial_projection = ProjectionUniform::from_view(
            world_size,
            UVec2::new(world_size.x as u32, world_size.y as u32),
        );
        let projection_uniform =
            draw_list_renderer.create_uniform("spline_projection", &initial_projection);

        let material = draw_list_renderer
            .create_material_from_shader("spline", SHADER)
            .uniform(0, 0, projection_uniform);
        let material = draw_list_renderer.create_material(material);

        Spline {
            draw_list_renderer,
            mesh,
            material,
            projection_uniform,
            world_size,
            pending_projection: None,
        }
    }
}

impl Scene for Spline {
    fn window_event(&mut self, event: &WindowEvent) {
        if let WindowEvent::Resized(size) = event {
            self.pending_projection = Some(ProjectionUniform::from_view(
                self.world_size,
                UVec2::new(size.width, size.height),
            ));
        }
    }

    fn frame(&mut self, _renderer: &Renderer, frame: &Frame, _delta_time: f32) {
        let mut draw_list = DrawList::new();

        if let Some(projection) = self.pending_projection.take() {
            draw_list.update_uniform(self.projection_uniform, &projection);
        }

        let instances = [Instance { offset: Vec2::ZERO }];
        draw_list.draw_mesh_instanced(RenderTarget::Surface, self.mesh, self.material, &instances);
        self.draw_list_renderer.submit_draw_list(
            FrameContext::new(
                &frame.view,
                UVec2::from(frame.surface_size),
                frame.surface_format,
            ),
            &draw_list,
        );
    }
}

fn main() {
    granite::run(SplineBuilder);
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
