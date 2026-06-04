precision mediump float;

uniform mat4 u_model;
uniform mat4 u_view;
uniform mat4 u_projection;
uniform float u_outline_width;

in vec2 a_corner;

in vec3 i_start;
in vec3 i_end;
in float i_radius;

out vec3 v_start_view;
out vec3 v_end_view;
out vec3 v_billboard_view;
out float v_radius;

void main() {
    vec3 start_view = (u_view * u_model * vec4(i_start, 1.0)).xyz;
    vec3 end_view = (u_view * u_model * vec4(i_end, 1.0)).xyz;
    float signed_radius = a_corner.x * max(i_radius + max(u_outline_width, 0.0), 0.0);
    float pushback = max(u_outline_width, 0.0) * 10.0;
    bool perspective = u_projection[3][3] == 0.0;

    if (perspective) {
        vec3 mid_before = length(end_view) < length(start_view) ? end_view : start_view;
        vec3 mid_after = mid_before;
        float mid_len = length(mid_before);
        if (mid_len > 1.0e-6) {
            mid_after += mid_before / mid_len * pushback;
        }

        float start_len = length(start_view);
        float end_len = length(end_view);
        if (start_len > 1.0e-6) {
            start_view += start_view / start_len * pushback;
        }
        if (end_len > 1.0e-6) {
            end_view += end_view / end_len * pushback;
        }

        vec4 mid_before_clip = u_projection * vec4(mid_before, 1.0);
        vec4 mid_before_radius_clip =
            u_projection * vec4(mid_before + vec3(signed_radius, 0.0, 0.0), 1.0);
        vec4 mid_after_clip = u_projection * vec4(mid_after, 1.0);
        vec4 mid_after_radius_clip =
            u_projection * vec4(mid_after + vec3(signed_radius, 0.0, 0.0), 1.0);
        mid_before_clip /= mid_before_clip.w;
        mid_before_radius_clip /= mid_before_radius_clip.w;
        mid_after_clip /= mid_after_clip.w;
        mid_after_radius_clip /= mid_after_radius_clip.w;

        float before_width = mid_before_radius_clip.x - mid_before_clip.x;
        float after_width = mid_after_radius_clip.x - mid_after_clip.x;
        if (abs(after_width) > 1.0e-6) {
            signed_radius *= abs(before_width / after_width);
        }
    } else {
        start_view.z -= pushback;
        end_view.z -= pushback;
    }

    vec3 axis = end_view - start_view;
    float axis_len = length(axis);
    float radius = abs(signed_radius);

    if (axis_len <= 1.0e-6 || radius <= 0.0) {
        v_start_view = start_view;
        v_end_view = end_view;
        v_billboard_view = start_view;
        v_radius = 0.0;
        gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
        return;
    }

    vec3 anchor = length(start_view) > length(end_view) ? end_view : start_view;
    vec3 plane_normal = perspective ? normalize(anchor) : vec3(0.0, 0.0, -1.0);
    vec3 billboard_view = anchor;
    float endpoint_sign = 1.0;

    if (a_corner.y < 0.0) {
        if (perspective) {
            vec3 endpoint_ray = normalize(start_view);
            float denom = dot(endpoint_ray, plane_normal);
            if (abs(denom) > 1.0e-6) {
                float t = dot(anchor - start_view, plane_normal) / denom;
                billboard_view = start_view + t * endpoint_ray;
            } else {
                billboard_view = start_view;
            }
        } else {
            billboard_view = start_view;
        }
    } else {
        endpoint_sign = -1.0;
        if (perspective) {
            vec3 endpoint_ray = normalize(end_view);
            float denom = dot(endpoint_ray, plane_normal);
            if (abs(denom) > 1.0e-6) {
                float t = dot(anchor - end_view, plane_normal) / denom;
                billboard_view = end_view + t * endpoint_ray;
            } else {
                billboard_view = end_view;
            }
        } else {
            billboard_view = end_view;
        }
    }

    vec3 side = perspective
        ? cross(billboard_view, axis)
        : cross(vec3(0.0, 0.0, -1.0), axis);
    float side_len = length(side);
    if (side_len <= 1.0e-6) {
        side = cross(vec3(0.0, 1.0, 0.0), axis);
        side_len = length(side);
    }
    if (side_len <= 1.0e-6) {
        side = cross(vec3(1.0, 0.0, 0.0), axis);
        side_len = length(side);
    }
    side = side / side_len * signed_radius;

    vec3 end_padding = perspective
        ? cross(billboard_view, side)
        : cross(vec3(0.0, 0.0, -1.0), side);
    float end_padding_len = length(end_padding);
    if (end_padding_len > 1.0e-6) {
        end_padding = end_padding / end_padding_len * signed_radius;
    } else {
        end_padding = vec3(0.0);
    }

    vec3 billboard_offset = endpoint_sign * 1.1 * (side + end_padding);
    billboard_view.xy += billboard_offset.xy;

    v_start_view = start_view;
    v_end_view = end_view;
    v_radius = radius;
    v_billboard_view = billboard_view;

    gl_Position = u_projection * vec4(billboard_view, 1.0);
}
