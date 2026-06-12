precision mediump float;

uniform mat4 u_projection;
uniform vec3 u_outline_color;

in vec2 v_mapping;
in vec3 v_center_view;
in float v_radius;
in float v_outline_radius;

out vec4 FragColor;

float coverage_from_edge(float signed_distance) {
    float width = max(fwidth(signed_distance), 1.0e-6);
    return 1.0 - smoothstep(-width, width, signed_distance);
}

void main() {
    float dist_sq = dot(v_mapping, v_mapping);
    float outline_radius_sq = v_outline_radius * v_outline_radius;
    float dist = sqrt(dist_sq);
    float edge_distance = dist - v_outline_radius;
    float alpha = coverage_from_edge(edge_distance);

    if (alpha <= 0.0) {
        discard;
    }

    float pushback = max(v_outline_radius - v_radius, 1.0e-4) * 10.0;
    float z = sqrt(max(outline_radius_sq - dist_sq, 0.0)) - pushback;
    vec3 view_pos = v_center_view + vec3(v_mapping, z);
    vec4 clip_pos = u_projection * vec4(view_pos, 1.0);
    float ndc_depth = clip_pos.z / clip_pos.w;
    gl_FragDepth = ndc_depth * 0.5 + 0.5;

    FragColor = vec4(u_outline_color, alpha);
}
