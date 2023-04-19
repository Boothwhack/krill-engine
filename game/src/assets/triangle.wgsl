struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>
}

struct CameraUniform {
    transform: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4(input.position, 0.0, 1.0) + camera.transform;
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(@location(0) color: vec4<f32>) -> @location(0) vec4<f32> {
    return color; //vec4(45.0 / 255.0, 106.0 / 255.0, 206.0 / 255.0, 1.0);
}
