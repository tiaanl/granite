use glam::{Mat4, UVec2, Vec3, Vec4};
use granite_draw::{
    DepthBufferId, DrawListRenderer, FrameContext, MaterialId, MeshId, ShaderVisibility, UniformId,
    depth_buffer::DepthBufferSize,
    draw_list::{DrawList, RenderTarget},
};
use granite_macros::{instance_buffer, uniform_buffer, vertex_buffer};

struct TerrainExample {
    draw_list_renderer: DrawListRenderer,

    chunk_mesh: MeshId,
    camera_uniform: UniformId,
    terrain_material: MaterialId,
    depth_buffer: DepthBufferId,
    chunk_instances: Vec<ChunkInstance>,

    new_size: Option<UVec2>,
}

impl TerrainExample {
    pub fn new(renderer: &mut granite::renderer::Renderer) -> Self {
        let mut draw_list_renderer =
            DrawListRenderer::new(renderer.device.clone(), renderer.queue.clone());

        let chunk_mesh = {
            #[vertex_buffer]
            struct Vertex {
                pos: Vec3,
                normal: Vec3,
                tex_coord: Vec3,
            }

            let mut vertices = vec![];
            for y in 0..9i32 {
                for x in 0..9i32 {
                    let pos = Vec3::new(x as f32, y as f32, 0.0);
                    let normal = Vec3::Z;
                    let tex_coord = pos / 9.0;
                    vertices.push(Vertex {
                        pos,
                        normal,
                        tex_coord,
                    });
                }
            }

            let mut indices = vec![];
            for y in 0..8u32 {
                for x in 0..8u32 {
                    let tl = y * 9 + x;
                    let tr = tl + 1;
                    let bl = tl + 9;
                    let br = bl + 1;
                    indices.extend_from_slice(&[tl, bl, br, tl, br, tr]);
                }
            }

            draw_list_renderer.create_mesh("terrain_chunk", &vertices, &indices)
        };

        let height_map_buffer = {
            let height_map = include_bytes!("../assets/heightmap.raw");
            assert_eq!(height_map.len(), 128 * 128);

            let height = |x: i32, y: i32| -> f32 {
                let x = x.clamp(0, 127) as usize;
                let y = y.clamp(0, 127) as usize;
                height_map[y * 128 + x] as f32 / 255.0
            };

            let mut data = Vec::with_capacity(128 * 128);
            for y in 0..128i32 {
                for x in 0..128i32 {
                    let elevation = height(x, y) * 20.0;
                    let dx = (height(x + 1, y) - height(x - 1, y)) * 20.0;
                    let dy = (height(x, y + 1) - height(x, y - 1)) * 20.0;
                    let normal = Vec3::new(-dx, -dy, 2.0).normalize();
                    data.push(Vec4::new(normal.x, normal.y, normal.z, elevation));
                }
            }

            draw_list_renderer
                .create_storage_buffer("heightmap", &data)
                .unwrap()
        };

        let camera_uniform = draw_list_renderer
            .create_uniform("camera", &Camera::new(UVec2::from(renderer.surface_size())));

        let depth_buffer =
            draw_list_renderer.create_depth_buffer("main", DepthBufferSize::SurfaceSize);

        let terrain_material = {
            let shader =
                draw_list_renderer.create_shader("terrain", include_str!("../assets/terrain.wgsl"));

            let vertex_shader = draw_list_renderer.create_vertex_shader(shader, "vertex");
            let fragment_shader = draw_list_renderer.create_fragment_shader(shader, "fragment");

            let material = granite_draw::Material::new(vertex_shader, fragment_shader)
                .depth_buffer(depth_buffer, granite_draw::DepthCompare::GreaterEqual)
                .uniform(0, 0, camera_uniform)
                .storage_buffer(0, 1, height_map_buffer, ShaderVisibility::Vertex);
            draw_list_renderer.create_material(material)
        };

        let chunk_instances: Vec<ChunkInstance> = (0..16)
            .flat_map(|y| (0..16).map(move |x| ChunkInstance { index: [x, y] }))
            .collect();

        Self {
            draw_list_renderer,
            chunk_mesh,
            camera_uniform,
            terrain_material,
            depth_buffer,
            chunk_instances,
            new_size: None,
        }
    }
}

#[uniform_buffer(Vertex)]
struct Camera {
    pub proj_view: glam::Mat4,
}

#[instance_buffer]
struct ChunkInstance {
    index: [u32; 2],
}

impl Camera {
    fn new(size: UVec2) -> Self {
        let aspect = size.x.max(1) as f32 / size.y.max(1) as f32;
        let fov = 60.0_f32.to_radians();

        let proj = Mat4::perspective_infinite_rh(fov, aspect, 0.1);
        let view = Mat4::look_at_rh(
            Vec3::new(64.0, -40.0, 40.0),
            Vec3::new(64.0, 64.0, 0.0),
            Vec3::Z,
        );

        Self {
            proj_view: proj * view,
        }
    }
}

impl granite::scene::Scene for TerrainExample {
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
        let mut draw_list = DrawList::default();

        if let Some(new_size) = self.new_size.take() {
            draw_list.update_uniform(self.camera_uniform, &Camera::new(new_size));
        }

        draw_list.clear_depth_buffer(self.depth_buffer, 0.0);

        draw_list.draw_mesh_instanced(
            RenderTarget::Surface,
            self.chunk_mesh,
            self.terrain_material,
            &self.chunk_instances,
        );

        self.draw_list_renderer.submit_draw_list(
            FrameContext {
                view: &frame.view,
                size: UVec2::from(frame.surface_size),
                format: frame.surface_format,
            },
            &draw_list,
        );
    }
}

fn main() {
    granite::run(TerrainExample::new);
}
