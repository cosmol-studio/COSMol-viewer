precision highp float;
precision highp int;

uniform highp sampler2D u_texture;
uniform bool u_outline;
uniform ivec2 u_origin;
uniform vec3 u_color;
uniform float u_width;
uniform float u_depth_jump;
uniform vec2 u_projection_depth;

#ifdef USE_SCOPED_SILHOUETTES
uniform highp usampler2D u_style_tags;
uniform highp sampler2D u_styles;
uniform int u_style_columns;
uniform float u_scene_depth;
#endif

out vec4 frag_color;

float eye_depth(float depth) {
    // Invert the depth component of the perspective projection used to draw the scene.
    return u_projection_depth.y / (2.0 * depth - 1.0 + u_projection_depth.x);
}

void main() {
    ivec2 pixel = ivec2(gl_FragCoord.xy) - u_origin;
    if (!u_outline) {
        frag_color = texelFetch(u_texture, pixel, 0);
        return;
    }

    ivec2 last = textureSize(u_texture, 0) - ivec2(1);
    float center = texelFetch(u_texture, pixel, 0).r;
    float nearest = center;
    vec3 color = u_color;
    int radius = int(u_width);
    float radius_squared = u_width * u_width;
    // Use a disk-shaped neighborhood and draw on the farther side of a depth edge.
    for (int y = -radius; y <= radius; ++y) {
        for (int x = -radius; x <= radius; ++x) {
            float distance_squared = float(x * x + y * y);
            if (distance_squared > radius_squared) continue;
            ivec2 sample_pixel = clamp(pixel + ivec2(x, y), ivec2(0), last);
            float depth = texelFetch(u_texture, sample_pixel, 0).r;
            if (depth >= nearest) continue;
#ifdef USE_SCOPED_SILHOUETTES
            // The near side owns the line, including edges over the background.
            int tag = int(texelFetch(u_style_tags, sample_pixel, 0).r);
            ivec2 entry = ivec2(2 * (tag % u_style_columns), tag / u_style_columns);
            vec4 style = texelFetch(u_styles, entry, 0);
            if (style.a == 0.0 || distance_squared > style.a * style.a) continue;
            if (center < 1.0) {
                float jump = texelFetch(u_styles, entry + ivec2(1, 0), 0).r * u_scene_depth;
                if (eye_depth(center) - eye_depth(depth) < jump) continue;
            }
            color = style.rgb;
#endif
            nearest = depth;
        }
    }

    if (nearest >= center) discard;
#ifndef USE_SCOPED_SILHOUETTES
    if (center < 1.0 && eye_depth(center) - eye_depth(nearest) < u_depth_jump) discard;
#endif
    frag_color = vec4(color, 1.0);
}
