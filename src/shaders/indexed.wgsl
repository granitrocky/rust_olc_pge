// Vertex shader

struct VertexInput {
    [[location(0)]] v_position: vec3<f32>;
    [[location(1)]] v_tex_coords: vec3<f32>;
    [[location(2)]] v_normal: vec3<f32>;
    [[location(3)]] v_color: vec4<f32>;
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
[[group(0), binding(1)]]
var r_sampler: sampler;

[[group(1), binding(0)]]
var r_texture: texture_2d<f32>;

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] vertex_normal: vec3<f32>;
    [[location(1)]] vertex_color: vec4<f32>;
    [[location(2)]] real_position: vec4<f32>;
    [[location(3)]] tex_coords: vec3<f32>;
};

[[stage(vertex)]]
fn vs_main(
           in_vertex: VertexInput
           ) -> VertexOutput {
    var out: VertexOutput;

    out.clip_position = uniforms.camera_transform * vec4<f32>(in_vertex.v_position, 1.0);
    out.real_position = vec4<f32>(in_vertex.v_position, 1.0);
    out.vertex_normal = in_vertex.v_normal;
    out.tex_coords = in_vertex.v_tex_coords;

    //For some reason the colors in WebGL are not normalized to 1.0, so this fixes that
    if(in_vertex.v_color.r > 1.0 || in_vertex.v_color.g > 1.0 || in_vertex.v_color.b > 1.0 || in_vertex.v_color.a > 1.0){
        out.vertex_color = in_vertex.v_color / 255.0;
    } else {
        out.vertex_color = in_vertex.v_color;
    }

    return out;
}

[[stage(fragment)]]
fn fs_main(
           in: VertexOutput,
           //[[builtin(position)]] frag_position: vec4<f32>
           ) -> [[location(0)]] vec4<f32>{
    let ls = normalize(uniforms.camera_position - in.real_position.xyz);
    let angle = max(dot(ls, normalize(in.vertex_normal)), 0.0);
    let color = textureSample(r_texture, r_sampler, in.tex_coords.xy);
    // + (in.vertex_color.xyz * angle)
    return color;// + (in.vertex_color * 0.05);
}
