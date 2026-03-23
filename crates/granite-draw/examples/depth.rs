use std::f32::consts::{PI, TAU};

use glam::{Mat4, UVec2, Vec3};
use granite::renderer::Renderer;
use granite_draw::{
    DepthBufferId, DepthCompare, DrawListRenderer, FrameContext, MaterialId, MeshId, UniformId,
    depth_buffer::DepthBufferSize,
    draw_list::{DrawList, RenderTarget},
};
use granite_macros::{instance_buffer, uniform_buffer, vertex_buffer};

const SHADER: &str = r"
struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct InstanceIn {
    @location(2) position: vec3<f32>,
}

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct Camera {
    proj: mat4x4<f32>,
    view: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

@vertex
fn vertex(instance: InstanceIn, vertex: VertexIn) -> VertexOut {
    let world_position = instance.position + vertex.position;

    let clip = camera.proj * camera.view * vec4<f32>(world_position, 1.0);

    return VertexOut(
        clip,
        world_position,
        vertex.normal,
    );
}

@fragment
fn fragment(vertex: VertexOut) -> @location(0) vec4<f32> {
    let light_position = vec3<f32>(-8.0, 8.0, -6.0);
    let light_direction = normalize(light_position - vertex.position);
    let normal = normalize(vertex.normal);

    let diffuse = max(dot(normal, light_direction), 0.0);
    let ambient = 0.2;
    let lighting = ambient + diffuse * 0.8;

    let base_color = vec3<f32>(0.95, 0.55, 0.25);
    let color = base_color * lighting;
    return vec4<f32>(color, 1.0);
}
";

#[uniform_buffer(VertexFragment)]
struct Camera {
    proj: Mat4,
    view: Mat4,
}

const WORLD_SIZE: f32 = 5.0;

impl Camera {
    fn new(size: UVec2) -> Self {
        const POSITION: Vec3 = Vec3 {
            x: 0.0,
            y: 0.0,
            z: 12.0,
        };

        const FOV: f32 = 60.0_f32.to_radians();

        let aspect = size.x.max(1) as f32 / size.y.max(1) as f32;

        Self {
            proj: Mat4::perspective_infinite_lh(FOV, aspect, 0.1),
            view: Mat4::from_translation(POSITION),
        }
    }
}

#[vertex_buffer]
struct Vertex {
    position: Vec3,
    normal: Vec3,
}

#[instance_buffer]
struct Ball {
    position: Vec3,
}

struct DepthExample {
    draw_list_renderer: DrawListRenderer,

    balls: Vec<Ball>,

    mesh: MeshId,
    camera: UniformId,
    material: MaterialId,
    depth_buffer: DepthBufferId,

    new_size: Option<UVec2>,
}

impl DepthExample {
    fn new(renderer: &mut Renderer) -> Self {
        let mut draw_list_renderer =
            DrawListRenderer::new(renderer.device.clone(), renderer.queue.clone());

        let depth_buffer =
            draw_list_renderer.create_depth_buffer("depth", DepthBufferSize::SurfaceSize);

        let (vertices, indices) =
            create_sphere_mesh(rand::random_range(1.0..WORLD_SIZE / 3.0), 32, 16);

        let mesh =
            draw_list_renderer.create_mesh("sphere", vertices.as_slice(), indices.as_slice());

        let camera = draw_list_renderer
            .create_uniform("camera", &Camera::new(UVec2::from(renderer.surface_size())));

        let material = draw_list_renderer
            .create_material_from_shader("main", SHADER)
            .depth_buffer(depth_buffer, DepthCompare::LessEqual)
            .uniform(0, 0, camera);
        let material = draw_list_renderer.create_material(material);

        let balls = (0..20)
            .map(|_| Ball {
                position: Vec3::new(
                    rand::random_range(-WORLD_SIZE..WORLD_SIZE),
                    rand::random_range(-WORLD_SIZE..WORLD_SIZE),
                    rand::random_range(-WORLD_SIZE..WORLD_SIZE),
                ),
            })
            .collect();

        Self {
            draw_list_renderer,

            balls,

            mesh,
            camera,
            material,
            depth_buffer,

            new_size: None,
        }
    }
}

impl granite::scene::Scene for DepthExample {
    fn window_event(&mut self, event: &granite::WindowEvent) {
        if let granite::WindowEvent::Resized(size) = event {
            self.new_size = Some(UVec2::new(size.width, size.height));
        }
    }

    fn frame(
        &mut self,
        _renderer: &granite::renderer::Renderer,
        frame: &granite::renderer::Frame,
        _delta_time: f32,
    ) {
        let surface_size = UVec2::from(frame.surface_size);

        let mut draw_list = DrawList::default();

        if let Some(new_size) = self.new_size.take() {
            draw_list.update_uniform(self.camera, &Camera::new(new_size));
        }

        draw_list.clear_depth_buffer(self.depth_buffer, 1.0);
        draw_list.draw_mesh_instanced(RenderTarget::Surface, self.mesh, self.material, &self.balls);

        self.draw_list_renderer.submit_draw_list(
            FrameContext::new(&frame.view, surface_size, frame.surface_format),
            &draw_list,
        );
    }
}

fn main() {
    granite::run(|renderer: &mut Renderer| DepthExample::new(renderer));
}

fn create_sphere_mesh(
    radius: f32,
    longitude_segments: u32,
    latitude_segments: u32,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices =
        Vec::with_capacity(((longitude_segments + 1) * (latitude_segments + 1)) as usize);
    let mut indices = Vec::with_capacity((longitude_segments * latitude_segments * 6) as usize);

    for latitude in 0..=latitude_segments {
        let v = latitude as f32 / latitude_segments as f32;
        let theta = v * PI;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for longitude in 0..=longitude_segments {
            let u = longitude as f32 / longitude_segments as f32;
            let phi = u * TAU;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            let normal = Vec3::new(sin_theta * cos_phi, cos_theta, sin_theta * sin_phi).normalize();
            vertices.push(Vertex {
                position: normal * radius,
                normal,
            });
        }
    }

    let stride = longitude_segments + 1;
    for latitude in 0..latitude_segments {
        for longitude in 0..longitude_segments {
            let i0 = latitude * stride + longitude;
            let i1 = i0 + 1;
            let i2 = i0 + stride;
            let i3 = i2 + 1;

            if latitude != 0 {
                indices.extend_from_slice(&[i0, i2, i1]);
            }
            if latitude != latitude_segments - 1 {
                indices.extend_from_slice(&[i1, i2, i3]);
            }
        }
    }

    (vertices, indices)
}
