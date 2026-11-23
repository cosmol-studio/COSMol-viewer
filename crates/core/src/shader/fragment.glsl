precision mediump float;

uniform vec3 u_light_pos;
uniform vec3 u_light_color;
uniform vec3 u_view_pos;
uniform float u_light_intensity;
uniform vec3 u_ambient_light_color;
uniform float u_ambient_light_intensity;
uniform float u_diffuse_light_intensity;
uniform float u_specular_light_intensity;
uniform int u_render_pass;
uniform int u_depth_cue_enabled;
uniform vec3 u_depth_cue_color;
uniform vec2 u_depth_cue_range;

in vec3 v_normal;
in vec3 v_frag_pos;
in vec3 v_eye_pos;
in vec4 v_color;
in vec2 v_material;
flat in highp uint v_silhouette_style;

#ifdef GL_ES
layout(location = 0) out vec4 FragColor;
layout(location = 1) out highp uint SilhouetteStyle;
#else
out vec4 FragColor;
out highp uint SilhouetteStyle;
#endif

#ifdef USE_MULTISHADOW
in highp vec3 v_shadow_pos;
uniform highp sampler2DShadow u_shadow_atlas;
uniform int u_shadow_count;
uniform highp float u_shadow_normal_bias;

struct ShadowSample {
    highp mat4 object_to_atlas;
    vec4 direction;
};
layout(std140) uniform MultishadowSamples {
    ShadowSample u_shadow_samples[MAX_SHADOW_DIRECTIONS];
};

float ambient_visibility(vec3 normal) {
    highp vec4 point = vec4(v_shadow_pos + normal * u_shadow_normal_bias, 1.0);
    float visible = 0.0;
    float total = 0.0;
    for (int i = 0; i < u_shadow_count; ++i) {
        float weight = dot(normal, u_shadow_samples[i].direction.xyz);
        if (weight > 0.0) {
            highp vec3 uv = (u_shadow_samples[i].object_to_atlas * point).xyz;
            visible += weight * texture(u_shadow_atlas, uv);
            total += weight;
        }
    }
    // Normalize by the unoccluded light, so a finite sample count cannot darken an open surface.
    return total > 0.0 ? visible / total : 1.0;
}
#endif

void main() {
    const float opaque_alpha_threshold = 0.99;
    bool transparent = v_color.a < opaque_alpha_threshold;

    if (v_color.a <= 0.0) {
        discard;
    }

    if (u_render_pass == 0 && transparent) {
        discard;
    }
    if (u_render_pass != 0 && !transparent) {
        discard;
    }

    SilhouetteStyle = v_silhouette_style;

    // The transparent depth pass only records the nearest transparent layer.
    if (u_render_pass == 1) {
        FragColor = vec4(0.0);
        return;
    }

    // Normalize once
    vec3 N = normalize(v_normal);
    vec3 L = normalize(u_light_pos - v_frag_pos);
    vec3 V = normalize(u_view_pos - v_frag_pos);
    float roughness = clamp(v_material.x, 0.04, 1.0);
    float metallic = clamp(v_material.y, 0.0, 1.0);
    vec3 base_color = v_color.rgb;

    // === Ambient ===
    vec3 ambient = u_ambient_light_intensity * u_ambient_light_color * base_color;
#ifdef USE_MULTISHADOW
    ambient *= ambient_visibility(N);
#endif

    // === Diffuse ===
    float diff = max(dot(N, L), 0.0);
    vec3 diffuse_color = mix(base_color, base_color * 0.2, metallic);
    vec3 diffuse = u_diffuse_light_intensity * diff * u_light_color * diffuse_color;

    // Roughness controls highlight width. Metallic tints the specular lobe.
    vec3 H = normalize(L + V);
    float shininess = mix(128.0, 10.0, roughness);
    float spec = pow(max(dot(N, H), 0.0), shininess);
    float spec_strength = mix(0.45, 0.08, roughness);
    vec3 specular_color = mix(vec3(1.0), base_color, metallic);
    vec3 specular = u_specular_light_intensity * spec_strength * spec * u_light_color * specular_color;

    // === Final Color ===
    vec3 lighting = ambient + diffuse + specular;
    vec3 final_color = lighting * u_light_intensity;

    if (u_depth_cue_enabled != 0) {
        float d = -v_eye_pos.z;
        float dim = (u_depth_cue_range.y - d) / (u_depth_cue_range.y - u_depth_cue_range.x);
        final_color = mix(u_depth_cue_color, final_color, clamp(dim, 0.0, 1.0));
    }

    FragColor = vec4(final_color, v_color.a);
}
