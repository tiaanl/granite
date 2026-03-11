//! Demonstrates custom render targets by rendering a colored triangle off-screen,
//! then sampling it in a grayscale post-processing pass to the surface.
//!
//! Pass 1: colored triangle → offscreen render target (RenderTarget::Custom)
//! Pass 2: fullscreen blit of render target → surface (RenderTarget::Surface)
use glam::{UVec2, Vec4};
use granite::{
    app::SceneBuilder,
    renderer::{Frame, Renderer},
    scene::Scene,
};
use granite_draw::{
    BlendMode, DrawListRenderer, FrameContext, MaterialId, MeshId, RenderTargetId,
    draw_list::{DrawList, RenderTarget},
    render_target::{RenderTargetFormat, RenderTargetSize},
    sampler::{SamplerAddressing, SamplerFiltering},
};
use granite_macros::vertex_buffer;

// Pass 1: draw a simple RGB triangle into the offscreen target.
const SCENE_SHADER: &str = r"
struct VertexIn {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
}

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex fn vertex(v: VertexIn) -> VertexOut {
    return VertexOut(vec4<f32>(v.position.xy, 0.0, 1.0), v.color);
}

@fragment fn fragment(v: VertexOut) -> @location(0) vec4<f32> {
    return v.color;
}
";

// Pass 2: sample the render target and convert to grayscale.
const POST_SHADER: &str = r"
@group(0) @binding(0) var t_color: texture_2d<f32>;
@group(0) @binding(1) var s_color: sampler;

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

// Generate a fullscreen triangle from vertex index — no vertex buffer needed.
@vertex fn vertex(@builtin(vertex_index) idx: u32) -> VertexOut {
    let uv = vec2<f32>(f32((idx << 1u) & 2u), f32(idx & 2u));
    let clip = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    // Flip V: NDC y-up vs texture y-down.
    return VertexOut(clip, vec2<f32>(uv.x, 1.0 - uv.y));
}

@fragment fn fragment(v: VertexOut) -> @location(0) vec4<f32> {
    let color = textureSample(t_color, s_color, v.uv);
    let luma = dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114));
    return vec4<f32>(luma, luma, luma, 1.0);
}
";

#[vertex_buffer]
struct Vertex {
    position: Vec4,
    color: Vec4,
}

struct PostProcessBuilder;

struct PostProcess {
    draw_list_renderer: DrawListRenderer,
    // Pass 1
    render_target: RenderTargetId,
    scene_mesh: MeshId,
    scene_material: MaterialId,
    // Pass 2
    post_material: MaterialId,
}

impl SceneBuilder for PostProcessBuilder {
    type Target = PostProcess;

    fn build(&self, renderer: &mut Renderer) -> Self::Target {
        let mut draw_list_renderer =
            DrawListRenderer::new(renderer.device.clone(), renderer.queue.clone());
        // Offscreen target that automatically matches and tracks the surface resolution.
        let render_target = draw_list_renderer.create_render_target(
            "offscreen",
            RenderTargetSize::SurfaceSize,
            RenderTargetFormat::Rgba,
        );

        // Pass 1 — colored triangle
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
        let scene_mesh = draw_list_renderer.create_mesh("triangle", vertices, &[0, 1, 2]);
        let scene_material = draw_list_renderer
            .create_material_from_shader("scene", SCENE_SHADER)
            .blend_mode(BlendMode::Opaque)
            .build();

        // Pass 2 — fullscreen grayscale blit
        let sampler = draw_list_renderer.create_sampler(
            "post",
            SamplerAddressing::ClampToEdge,
            SamplerFiltering::Linear,
        );
        let post_material = draw_list_renderer
            .create_material_from_shader("post", POST_SHADER)
            .render_target_texture(0, 0, render_target)
            .sampler(0, 1, sampler)
            .blend_mode(BlendMode::Opaque)
            .build();

        PostProcess {
            draw_list_renderer,
            render_target,
            scene_mesh,
            scene_material,
            post_material,
        }
    }
}

impl Scene for PostProcess {
    fn frame(&mut self, _renderer: &Renderer, frame: &Frame, _delta_time: f32) {
        let mut draw_list = DrawList::new();

        // Pass 1: draw the triangle into the offscreen render target.
        draw_list.draw_mesh(
            RenderTarget::Custom(self.render_target),
            self.scene_mesh,
            self.scene_material,
        );

        // Pass 2: blit the render target to the surface as grayscale.
        // 3 vertices generate the fullscreen triangle in the vertex shader.
        draw_list.draw(RenderTarget::Surface, self.post_material, 3);

        self.draw_list_renderer.submit_draw_list(
            FrameContext::new(
                &frame.view,
                UVec2::from(frame.surface_size),
                frame.surface_format,
            ),
            draw_list,
        );
    }
}

fn main() {
    granite::run(PostProcessBuilder).unwrap();
}
