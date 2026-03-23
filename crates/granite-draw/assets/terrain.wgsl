struct Camera {
    proj_view: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(0) @binding(1)
var<storage> height_map: array<vec4<f32>>;


struct Vertex {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coord: vec3<f32>,
}

struct Instance {
    @location(3) chunk_index: vec2<u32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
}

@vertex
fn vertex(vertex: Vertex, instance: Instance) -> VertexOutput {
    let global_pos = vec2<u32>(vec2<f32>(vertex.pos.xy)) + instance.chunk_index * 8u;
    let clamped = min(global_pos, vec2<u32>(127u));
    let height_data = height_map[clamped.y * 128u + clamped.x];
    let world_pos = vec3<f32>(vec2<f32>(global_pos), height_data.w);

    return VertexOutput(
        camera.proj_view * vec4(world_pos, 1.0),
        height_data.xyz,
    );
}

const LIGHT_DIR: vec3<f32> = vec3<f32>(0.3, -0.5, 0.8);
const BASE_COLOR: vec3<f32> = vec3<f32>(0.4, 0.65, 0.3);

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(input.normal);
    let l = normalize(LIGHT_DIR);
    let diffuse = max(dot(n, l), 0.0);
    let ambient = 0.15;
    let color = BASE_COLOR * (ambient + diffuse);
    return vec4(color, 1.0);
}
