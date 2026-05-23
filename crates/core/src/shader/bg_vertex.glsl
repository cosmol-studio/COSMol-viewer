precision mediump float;

uniform vec4 background_color;

const vec2 verts[6] = vec2[6](
    vec2(-1.0, 1.0),
    vec2(-1.0, -1.0),
    vec2(1.0, 1.0),
    vec2(1.0, 1.0),
    vec2(-1.0, -1.0),
    vec2(1.0, -1.0)
);
out vec4 v_color;
void main() {
    v_color = background_color;
    gl_Position = vec4(verts[gl_VertexID], 0.0, 1.0);
}
