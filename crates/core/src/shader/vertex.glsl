precision mediump float;

uniform mat4 u_model;
uniform mat4 u_view;
uniform mat4 u_projection;
uniform mat3 u_normal_matrix;

in vec3 a_position;
in vec3 a_normal;
in vec4 a_color;
in vec2 a_material;

out vec3 v_normal;
out vec3 v_frag_pos;
out vec4 v_color;
out vec2 v_material;

void main() {
    // 1. 模型空间 -> 世界空间
    vec4 world_pos = u_model * vec4(a_position, 1.0);
    v_frag_pos = world_pos.xyz;

    // 2. 法线也要变换到世界空间
    v_normal = normalize(u_normal_matrix * a_normal);

    // 3. 把顶点变换到最终裁剪空间用于光栅化
    gl_Position = u_projection * u_view * world_pos;

    // 4. 传颜色
    v_color = a_color;
    v_material = a_material;
}
