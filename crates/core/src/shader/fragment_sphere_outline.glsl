precision mediump float;

uniform mat4 u_projection;
uniform vec3 u_outline_color;

in vec2 v_mapping;
in vec3 v_center_view;
in float v_radius;
in float v_outline_radius;

out vec4 FragColor;

void main() {
    float dist_sq = dot(v_mapping, v_mapping);
    float outline_radius_sq = v_outline_radius * v_outline_radius;

    if (dist_sq > outline_radius_sq) {
        discard;
    }

    float pushback = max(v_outline_radius - v_radius, 1.0e-4) * 10.0;
    float z = sqrt(max(outline_radius_sq - dist_sq, 0.0)) - pushback;
    vec3 view_pos = v_center_view + vec3(v_mapping, z);
    vec4 clip_pos = u_projection * vec4(view_pos, 1.0);
    float ndc_depth = clip_pos.z / clip_pos.w;
    gl_FragDepth = ndc_depth * 0.5 + 0.5;

    FragColor = vec4(u_outline_color, 1.0);
}
