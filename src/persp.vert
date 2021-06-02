// shader.vert
#version 450

layout(location=0) in vec3 a_position;
layout(location=1) in uint type;
layout(location=2) in float value;
layout(location=3) in float density;
layout(location=4) in float emission;
layout(location=5) in float absorption;

layout(set=0, binding=0)
uniform Uniforms {
    mat4 camera_transform;
    mat4 camera_inverse_transform;
    vec3 camera_position;
    float screen_width;
    float screen_height;
};

layout(location=0) out vec4 v_position;
layout(location=1) out float f_camera_dot;
layout(location=2) out float f_point_size;
void main(){
    vec4 pos = camera_transform * vec4(a_position, 1.0);
    f_camera_dot = dot(vec3(0.0, 0.0, 1.0), normalize(pos.xyz));
    gl_Position = pos;
    v_position = pos;
    gl_PointSize = value / density / length(pos.xyz);
    f_point_size = gl_PointSize;
}
