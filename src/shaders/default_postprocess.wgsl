 
 
struct VertexOutput{
    [[builtin(position)]] pos: vec4<f32>;
    [[location(0)]] tex_coord: vec2<f32>;
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


[[stage(vertex)]]
fn vs_main(
           [[location(0)]] in_position: vec3<f32>,
           [[location(1)]] in_tex_coord: vec2<f32>,
           ) -> VertexOutput
{
    var v_out: VertexOutput;
    v_out.pos = vec4<f32>(in_position, 1.0);
    v_out.tex_coord = in_tex_coord;
    return v_out;
}
[[stage(fragment)]]
fn fs_main( in: VertexOutput) -> [[location(0)]] vec4<f32>{
    let sample = textureSample(r_texture, r_sampler, in.tex_coord);
    return sample;
    // if(sample.w > 0.001) {return sample;}
    // else{ return vec4<f32>(0.0);}
}
