import bevy_sprite_render::{mesh2d::vertex_output::VertexOutput, sprite_mesh::functions as sprite_fn};

struct DissolveMaterial {
    burn_edge_color: vec4<f32>,
    amount: f32,
    burn_edge: f32,
    // WebGL2 requires structs to be aligned to 16 bytes
    @if(SIXTEEN_BYTE_ALIGNMENT)
    _webgl2_padding_12b: u32,
    @if(SIXTEEN_BYTE_ALIGNMENT)
    _webgl2_padding_16b: u32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(20) var<uniform> material: DissolveMaterial;
@group(constants::MATERIAL_BIND_GROUP) @binding(21) var noise_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(22) var noise_sampler: sampler;

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    var noise = textureSample(noise_texture, noise_sampler, input.uv).r;
    // Change the noise's range from 0..1 to burn_edge..1 - burn_edge
    // in order to avoid the burn edge showing up when the sprite is fully visible or fully hidden
    noise = mix(material.burn_edge, 1.0 - material.burn_edge, noise);

    // If this value is 0..burn_edge, the pixel is part of the burn edge
    // If it's >= burn_edge, the pixel is visible
    // Otherwise, it's hidden
    let difference = noise - material.amount;
    // Get the sprite's color
    let sprite_color = sprite_fn::sample_sprite_texture(input.uv, input.instance_index);

    var color = vec4(0.0);
    let burn_edge = difference >= 0 && difference < material.burn_edge;
    let visible = difference >= material.burn_edge;
    if burn_edge {
        color = material.burn_edge_color;
        color.a = sprite_color.a;
    } else if visible {
        color = sprite_color;
    }

    // Apply alpha discard and tint
    color = sprite_fn::get_final_color(color, input.instance_index);
    return color;
}
