precision mediump float;

uniform vec3 u_light_pos;
uniform vec3 u_light_color;
uniform vec3 u_view_pos;
uniform float u_light_intensity;
uniform int u_render_pass;
uniform int u_depth_cue_enabled;
uniform vec3 u_depth_cue_color;
uniform vec2 u_depth_cue_range;

in vec3 v_normal;
in vec3 v_frag_pos;
in vec3 v_eye_pos;
in vec4 v_color;
in vec2 v_material;

out vec4 FragColor;

void main() {
    const float opaque_alpha_threshold = 0.99;
    bool transparent = v_color.a < opaque_alpha_threshold;

    if (u_render_pass == 0 && transparent) {
        discard;
    }
    if (u_render_pass != 0 && !transparent) {
        discard;
    }

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
    vec3 ambient = 0.55 * u_light_color * base_color;

    // === Diffuse ===
    float diff = max(dot(N, L), 0.0);
    vec3 diffuse_color = mix(base_color, base_color * 0.2, metallic);
    vec3 diffuse = 0.65 * diff * u_light_color * diffuse_color;

    // Roughness controls highlight width. Metallic tints the specular lobe.
    vec3 H = normalize(L + V);
    float shininess = mix(128.0, 10.0, roughness);
    float spec = pow(max(dot(N, H), 0.0), shininess);
    float spec_strength = mix(0.45, 0.08, roughness);
    vec3 specular_color = mix(vec3(1.0), base_color, metallic);
    vec3 specular = spec_strength * spec * u_light_color * specular_color;

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
