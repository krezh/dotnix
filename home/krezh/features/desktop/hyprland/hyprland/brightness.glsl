// Screen shader to make display brighter and more vibrant (like Windows)
#version 300 es

precision highp float;

in vec2 v_texcoord;
out vec4 fragColor;
uniform sampler2D tex;

void main() {
    vec4 pixColor = texture(tex, v_texcoord);

    // Adjust these values to tune the display
    float brightness = 1.2;  // 1.0 = normal, higher = brighter
    float gamma = 1.00;        // higher = lighter midtones
    float saturation = 1.1;   // higher = more vibrant colors

    // Apply adjustments
    pixColor.rgb *= brightness;
    pixColor.rgb = pow(pixColor.rgb, vec3(1.0/gamma));

    float lum = dot(pixColor.rgb, vec3(0.299, 0.587, 0.114));
    pixColor.rgb = mix(vec3(lum), pixColor.rgb, saturation);

    pixColor.rgb = clamp(pixColor.rgb, 0.0, 1.0);

    fragColor = pixColor;
}
