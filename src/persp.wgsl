// Vertex shader

struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] type: u32;
    [[location(2)]] value: f32;
    [[location(3)]] density: f32;
    [[location(4)]] emission: f32;
    [[location(5)]] absorption: f32;
};

[[block]]
struct Uniforms{
    camera_transform: mat4x4<f32>;
    camera_inverse_transform: mat4x4<f32>;
    camera_position: vec3<f32>;
    screen_width: f32;
    screen_height: f32;
};

[[group(0), binding(0)]]
var uniforms: Uniforms;

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] f_point_size: f32;
};

[[stage(vertex)]]
fn vs_main(
           in_vertex: VertexInput,
           [[builtin(vertex_index)]] in_vertex_index: u32
) -> VertexOutput {
    var out: VertexOutput;
    var pos: vec4<f32> = uniforms.camera_transform * vec4<f32>(in_vertex.position, 1.0);
    out.f_point_size = in_vertex.value / in_vertex.density / length(pos.xyz);
    out.clip_position = pos;
    return out;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32>{
    return vec4<f32>(1.0,1.0,0.0,1.0);
}
