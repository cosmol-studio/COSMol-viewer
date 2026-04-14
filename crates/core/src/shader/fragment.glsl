precision mediump float;

uniform vec3 u_light_pos;
uniform vec3 u_light_color;
uniform vec3 u_view_pos;
uniform float u_light_intensity;

in vec3 v_normal;
in vec3 v_frag_pos;
in vec4 v_color;
in vec2 v_material;

out vec4 FragColor;

void main() {
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

    FragColor = vec4(lighting * u_light_intensity, v_color.a);
}
