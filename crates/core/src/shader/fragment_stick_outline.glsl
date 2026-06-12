precision mediump float;

uniform mat4 u_projection;
uniform vec3 u_outline_color;

in vec3 v_start_view;
in vec3 v_end_view;
in vec3 v_billboard_view;
in float v_radius;

out vec4 FragColor;

float coverage_from_implicit(float value) {
    float width = max(fwidth(value), 1.0e-6);
    return smoothstep(-width, width, value);
}

void main() {
    vec3 axis = v_end_view - v_start_view;
    float axis_len = length(axis);
    if (axis_len <= 1.0e-6 || v_radius <= 0.0) {
        discard;
    }

    vec3 ray_origin = v_billboard_view;
    vec3 ray_dir = vec3(0.0, 0.0, -1.0);
    if (u_projection[3][3] == 0.0) {
        ray_dir = normalize(v_billboard_view);
    }

    vec3 axis_dir = axis / axis_len;
    vec3 ray_perp = ray_dir - dot(ray_dir, axis_dir) * axis_dir;
    float a = dot(ray_perp, ray_perp);
    if (a <= 1.0e-8) {
        discard;
    }

    vec3 delta = ray_origin - v_start_view;
    vec3 delta_perp = delta - dot(delta, axis_dir) * axis_dir;
    float b = 2.0 * dot(ray_perp, delta_perp);
    float c = dot(delta_perp, delta_perp) - v_radius * v_radius;
    float det = b * b - 4.0 * a * c;
    float alpha = coverage_from_implicit(det);
    if (alpha <= 0.0) {
        discard;
    }

    float sqrt_det = sqrt(max(det, 0.0));
    float t0 = (-b - sqrt_det) / (2.0 * a);
    float t1 = (-b + sqrt_det) / (2.0 * a);
    float t = min(t0, t1);

    vec3 view_pos = ray_origin + ray_dir * t;
    float axis_pos = dot(axis_dir, view_pos - v_start_view);
    if (axis_pos < 0.0 || axis_pos > axis_len) {
        vec3 cap_center = axis_pos < 0.0 ? v_start_view : v_end_view;
        vec3 cap_delta = ray_origin - cap_center;
        float cap_a = dot(ray_dir, ray_dir);
        float cap_b = 2.0 * dot(cap_delta, ray_dir);
        float cap_c = dot(cap_delta, cap_delta) - v_radius * v_radius;
        float cap_det = cap_b * cap_b - 4.0 * cap_a * cap_c;
        alpha = coverage_from_implicit(cap_det);
        if (alpha <= 0.0) {
            discard;
        }

        float cap_sqrt_det = sqrt(max(cap_det, 0.0));
        float cap_t0 = (-cap_b - cap_sqrt_det) / (2.0 * cap_a);
        float cap_t1 = (-cap_b + cap_sqrt_det) / (2.0 * cap_a);
        t = min(cap_t0, cap_t1);
        view_pos = ray_origin + ray_dir * t;
    }

    vec4 clip_pos = u_projection * vec4(view_pos, 1.0);
    float ndc_depth = clip_pos.z / clip_pos.w;
    gl_FragDepth = ndc_depth * 0.5 + 0.5;

    FragColor = vec4(u_outline_color, alpha);
}
