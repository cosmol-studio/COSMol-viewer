precision mediump float;

uniform mat4 u_model;
uniform mat4 u_view;
uniform mat4 u_projection;
uniform float u_outline_width;

in vec2 a_corner;

in vec3 i_position;
in float i_radius;

out vec2 v_mapping;
out vec3 v_center_view;
out float v_radius;
out float v_outline_radius;

void main() {
    vec4 center_view4 = u_view * u_model * vec4(i_position, 1.0);
    vec3 center_view = center_view4.xyz;
    float radius = max(i_radius, 0.0);
    float outline_radius = radius + max(u_outline_width, 0.0);
    vec2 mapping = a_corner * outline_radius;

    v_mapping = mapping;
    v_center_view = center_view;
    v_radius = radius;
    v_outline_radius = outline_radius;

    vec4 view_pos = vec4(center_view + vec3(mapping, 0.0), 1.0);
    gl_Position = u_projection * view_pos;
}
