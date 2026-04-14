precision mediump float;

uniform mat4 u_model;
uniform mat4 u_view;
uniform mat4 u_projection;
uniform mat3 u_normal_matrix;

// per-vertex attributes (unit cylinder template)
in vec3 a_position;
in vec3 a_normal;

// per-instance attributes
in vec3 i_start;
in vec3 i_end;
in float i_radius;
in vec4 i_color;
in vec2 i_material;

out vec3 v_normal;
out vec3 v_frag_pos;
out vec4 v_color;
out vec2 v_material;

void main() {
    // 1️⃣ 棒方向和长度
    vec3 dir = i_end - i_start;
    float len = length(dir);
    vec3 z_axis = normalize(dir);

    // 2️⃣ 局部坐标系
    vec3 tmp = vec3(0.0, 1.0, 0.0);
    if (abs(dot(z_axis, tmp)) > 0.99) tmp = vec3(1.0, 0.0, 0.0); // 避免共线
    vec3 x_axis = normalize(cross(tmp, z_axis));
    vec3 y_axis = cross(z_axis, x_axis);
    mat3 rot = mat3(x_axis, y_axis, z_axis); // 列向量 = x, y, z

    // 3️⃣ 顶点缩放 + 旋转 + 平移
    vec3 local_pos = vec3(a_position.x * i_radius, a_position.y * i_radius, a_position.z * len);
    vec4 transformed = vec4(rot * local_pos + i_start, 1.0);

    // 4️⃣ 模型空间 -> 世界空间
    vec4 world_pos = u_model * transformed;
    v_frag_pos = world_pos.xyz;

    // 5️⃣ 法线变换到世界空间
    vec3 normal_world = rot * a_normal;
    v_normal = normalize(u_normal_matrix * normal_world);

    // 6️⃣ 输出颜色
    v_color = i_color;
    v_material = i_material;

    // 7️⃣ 投影
    gl_Position = u_projection * u_view * world_pos;
}
