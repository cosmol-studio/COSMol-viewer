precision mediump float;

uniform mat4 u_model;
uniform mat4 u_view;
uniform mat4 u_projection;
uniform float u_outline_width;

in vec3 a_position;

in vec3 i_position;
in float i_radius;

void main() {
    float outline_radius = i_radius + u_outline_width;
    vec4 local_pos = vec4(a_position * outline_radius + i_position, 1.0);
    vec4 world_pos = u_model * local_pos;
    gl_Position = u_projection * u_view * world_pos;
}
