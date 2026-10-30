@if(FIRST_MIP) {
@if(MULTISAMPLE) {
@group(0) @binding(0) var source_mip: texture_depth_multisampled_2d;
} @else {
@group(0) @binding(0) var source_mip: texture_depth_2d;
}
} @else {
@group(0) @binding(0) var source_mip: texture_2d<f32>;
}
@group(0) @binding(1) var destination_mip: texture_storage_2d<r32float, write>;

fn reduce_4(v: vec4f) -> f32 {
    return min(min(v.x, v.y), min(v.z, v.w));
}

@if(FIRST_MIP) {
// Match the original generator's conservative gather from a virtual
// power-of-two depth buffer. Explicit loads keep this a two-binding pass.
fn load_virtual_depth(position: vec2u) -> f32 {
    let actual_size = textureDimensions(source_mip);
    let virtual_size = textureDimensions(destination_mip) * 2u;
    let uv = (vec2f(position) + 0.5) / vec2f(virtual_size);
    let base = vec2i(floor(uv * vec2f(actual_size) - 0.5));
    let maximum = vec2i(actual_size) - 1;
    var result = 1.0;
@if(MULTISAMPLE) {
    let sample_count = i32(textureNumSamples(source_mip));
} @else {
    let sample_count = 1;
}
    for (var sample = 0; sample < sample_count; sample++) {
        let depths = vec4f(
            textureLoad(source_mip, clamp(base, vec2i(0), maximum), sample),
            textureLoad(source_mip, clamp(base + vec2i(1, 0), vec2i(0), maximum), sample),
            textureLoad(source_mip, clamp(base + vec2i(0, 1), vec2i(0), maximum), sample),
            textureLoad(source_mip, clamp(base + vec2i(1, 1), vec2i(0), maximum), sample),
        );
        result = min(result, reduce_4(depths));
    }
    return result;
}
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3u) {
    if any(id.xy >= textureDimensions(destination_mip)) {
        return;
    }
    let source = id.xy * 2u;
@if(FIRST_MIP) {
    let depth = reduce_4(vec4f(
        load_virtual_depth(source),
        load_virtual_depth(source + vec2u(1, 0)),
        load_virtual_depth(source + vec2u(0, 1)),
        load_virtual_depth(source + vec2u(1, 1)),
    ));
} @else {
    let maximum = textureDimensions(source_mip) - 1u;
    let depth = reduce_4(vec4f(
        textureLoad(source_mip, vec2i(min(source, maximum)), 0).r,
        textureLoad(source_mip, vec2i(min(source + vec2u(1, 0), maximum)), 0).r,
        textureLoad(source_mip, vec2i(min(source + vec2u(0, 1), maximum)), 0).r,
        textureLoad(source_mip, vec2i(min(source + vec2u(1, 1), maximum)), 0).r,
    ));
}
    textureStore(destination_mip, vec2i(id.xy), vec4f(depth));
}
