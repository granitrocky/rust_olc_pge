struct VertexOutput{
    [[builtin(position)]] pos: vec4<f32>;
    [[location(0)]] tex_coord: vec2<f32>;
};

[[group(0), binding(0)]]
var r_texture: texture_2d<f32>;
[[group(0), binding(1)]]
var r_sampler: sampler;

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
    return textureSample(r_texture, r_sampler, in.tex_coord);
}
