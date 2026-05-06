precision mediump float;

uniform mat4 u_model;
uniform mat4 u_view;
uniform mat4 u_projection;
uniform float u_outline_width;

in vec3 a_position;

in vec3 i_start;
in vec3 i_end;
in float i_radius;

void main() {
    vec3 dir = i_end - i_start;
    float len = length(dir);
    vec3 z_axis = normalize(dir);

    vec3 tmp = vec3(0.0, 1.0, 0.0);
    if (abs(dot(z_axis, tmp)) > 0.99) tmp = vec3(1.0, 0.0, 0.0);
    vec3 x_axis = normalize(cross(tmp, z_axis));
    vec3 y_axis = cross(z_axis, x_axis);
    mat3 rot = mat3(x_axis, y_axis, z_axis);

    float outline_radius = i_radius + u_outline_width;
    vec3 local_pos = vec3(
        a_position.x * outline_radius,
        a_position.y * outline_radius,
        a_position.z * len
    );
    vec4 transformed = vec4(rot * local_pos + i_start, 1.0);
    vec4 world_pos = u_model * transformed;
    gl_Position = u_projection * u_view * world_pos;
}
