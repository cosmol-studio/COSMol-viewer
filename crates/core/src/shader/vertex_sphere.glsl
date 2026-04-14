precision mediump float;

uniform mat4 u_model;
uniform mat4 u_view;
uniform mat4 u_projection;
uniform mat3 u_normal_matrix;

// per-vertex
in vec3 a_position;
in vec3 a_normal;

// per-instance
in vec3 i_position;  // sphere center
in float i_radius;   // sphere radius
in vec4 i_color;     // sphere color
in vec2 i_material;

out vec3 v_normal;
out vec3 v_frag_pos;
out vec4 v_color;
out vec2 v_material;

void main() {
    // 1️⃣ 顶点缩放 + 平移到实例中心
    vec4 local_pos = vec4(a_position * i_radius + i_position, 1.0);

    // 2️⃣ 模型空间 -> 世界空间
    vec4 world_pos = u_model * local_pos;
    v_frag_pos = world_pos.xyz;

    // 3️⃣ 法线变换到世界空间
    v_normal = normalize(u_normal_matrix * a_normal);

    // 4️⃣ 输出颜色
    v_color = i_color;
    v_material = i_material;

    // 5️⃣ 投影
    gl_Position = u_projection * u_view * world_pos;
}
