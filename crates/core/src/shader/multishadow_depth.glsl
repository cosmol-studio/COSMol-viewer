precision mediump float;

in vec4 v_color;

void main() {
    if (v_color.a < 0.99) {
        discard;
    }
}
