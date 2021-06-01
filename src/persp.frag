// shader.frag
#version 450

layout(location=0) in vec4 v_vertex_position;
layout(location=1) in float camera_dot;
layout(location=2) in float f_point_size;

layout(location=0) out vec4 f_color;

layout(set=0, binding=0)
uniform Uniforms {
    mat4 camera_transform;
    mat4 camera_inverse_transform;
    vec3 camera_position;
    float screen_width;
    float screen_height;
};

void main() {
    vec2 vertex_screen_normal = (v_vertex_position.xy / v_vertex_position.w + 1.0) / 2.0;
    vec2 frag_normal = vec2(gl_FragCoord.x / screen_width, gl_FragCoord.y / screen_height);
    vec2 vertex_screen_position = vec2(vertex_screen_normal.x * screen_width,
                                        vertex_screen_normal.y * screen_height);

    float aspect = screen_width / screen_height;
    frag_normal.y = 1.0 - frag_normal.y;
    float coord = length(
            vec2(gl_FragCoord.x, screen_height - gl_FragCoord.y) - vertex_screen_position
    );
    float alphay = 0.5 - sqrt(coord / f_point_size);
    vec3 color3 = alphay * vec3(1.0, 0.5, 0.5);
    f_color = vec4(
    color3,
    alphay);
    //texture(sampler2D(t_diffuse, s_diffuse),v_tex_coords);
}