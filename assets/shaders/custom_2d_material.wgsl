
struct Vertex {
    [[location(2)]] uv: vec2<f32>;
};
struct VertexOutput {
    [[location(0)]] uv: vec2<f32>;
};

struct CustomMaterial {
    color: vec4<f32>;
    size: vec2<f32>;
    transparencies: array<vec4<f32>, 5>;
    positions: array<vec4<f32>, 5>;
};

[[group(1), binding(0)]]
var<uniform> material: CustomMaterial;

[[stage(fragment)]]
fn fragment(in: VertexOutput) -> [[location(0)]] vec4<f32> {
 
    let uv_01 = in.uv / material.size.x  + 0.5;
    var color: vec4<f32> = material.color;

    for (var i: i32 = 0; i < 5; i = i+1 ) {

        if (uv_01.x >  material.positions[i][0] ) {
            
            color.w = material.transparencies[i][0];

        } 
    }

    return color;
}
