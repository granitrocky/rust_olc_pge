// shader.vert
#version 450

layout(location=0) in vec3 a_position;
layout(location=1) in vec2 a_TexCoord;

layout(location=0) out vec2 v_TexCoord;
void main(){
    v_TexCoord = a_TexCoord;
    gl_Position = vec4(a_position, 1.0);
}