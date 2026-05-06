precision mediump float;

uniform vec3 u_outline_color;

out vec4 FragColor;

void main() {
    FragColor = vec4(u_outline_color, 1.0);
}
