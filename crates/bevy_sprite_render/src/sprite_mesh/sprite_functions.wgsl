#define_import_path bevy_sprite::sprite_functions

#import bevy_sprite::{
    sprite_types::{
        SPRITE_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS,
        SPRITE_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE,
        SPRITE_MATERIAL_FLAGS_ALPHA_MODE_MASK,
        SPRITE_MATERIAL_FLAGS_FLIP_X,
        SPRITE_MATERIAL_FLAGS_FLIP_Y,
        SPRITE_MATERIAL_FLAGS_TILE_X,
        SPRITE_MATERIAL_FLAGS_TILE_Y,
    },
    sprite_bindings::{material, texture, texture_sampler},
}

// Applies all the transformations to the UV and samples the sprite's final color.
fn sample_final_color(uv: vec2<f32>) -> vec4<f32> {
    let sprite_color = sample_sprite_texture(uv);
    return get_final_color(sprite_color);
}

// Applies all the necessary transformations to the UV and samples the sprite's texture.
fn sample_sprite_texture(uv: vec2<f32>) -> vec4<f32> {
    let final_uv = get_final_uv(uv);
    return textureSample(texture, texture_sampler, final_uv);
}

// Applies the tint and alpha discard on the sprite color.
fn get_final_color(sprite_color: vec4<f32>) -> vec4<f32> {
    var output_color = apply_tint(sprite_color);
    output_color = alpha_discard(output_color);
    return output_color;
}

// Applies all the necessary transformations to get the final UV that the texture should be sampled from.
fn get_final_uv(uv: vec2<f32>) -> vec2<f32> {
    var out = uv;
    out = apply_flip(out);
    out = apply_tiling(out);
    out = apply_slicing(out);
    out = apply_uv_transform(out);
    return out;
}

// Flips the UV based on the sprite's flip X and Y properties.
fn apply_flip(uv: vec2<f32>) -> vec2<f32> {
    var out = uv;
    if (material.flags & SPRITE_MATERIAL_FLAGS_FLIP_X) != 0u {
        out.x = 1.0 - out.x;
    }
    if (material.flags & SPRITE_MATERIAL_FLAGS_FLIP_Y) != 0u {
        out.y = 1.0 - out.y;
    }

    return out;
}

// Applies tiling to the UV based on the sprite's tiling properties when `image_mode` is `Tiled`.
fn apply_tiling(uv: vec2<f32>) -> vec2<f32> {
    var out = uv;
    if (material.flags & SPRITE_MATERIAL_FLAGS_TILE_X) != 0u {
        out.x = (out.x - material.tile_stretch_value.x * floor(out.x / material.tile_stretch_value.x)) / material.tile_stretch_value.x;
    }
    if (material.flags & SPRITE_MATERIAL_FLAGS_TILE_Y) != 0u {
        out.y = (out.y - material.tile_stretch_value.y * floor(out.y / material.tile_stretch_value.y)) / material.tile_stretch_value.y;
    }

    return out;
}

// Applies the sprite's UV transform,
// which is used for sampling the correct region from a texture atlas
// and scaling the sprite when `image_mode` is `Scaled`.
fn apply_uv_transform(uv: vec2<f32>) -> vec2<f32> { 
    return (material.uv_transform * vec3(uv, 1.0)).xy;
}

// Applies UV slicing based on the sprite's slicing properties when `image_mode` is `Sliced`.
fn apply_slicing(uv: vec2<f32>) -> vec2<f32> {
    // using this as a temp check for slicing
    if material.scale.x == 0.0 {
        return uv;
    }

    let min_inset_scaled = material.min_inset / material.scale;
    let max_inset_scaled = material.max_inset / material.scale;

    let left = uv.x < min_inset_scaled.x;
    let right = uv.x > 1.0 - max_inset_scaled.x;
    let top = uv.y < min_inset_scaled.y;
    let bottom = uv.y > 1.0 - max_inset_scaled.y;

    // top-left corner
    if top && left {
        return uv * material.scale;
    }

    // top-right corner
    if top && right {
        return vec2<f32>(
            1.0 - (1.0 - uv.x) * material.scale.x,
            uv.y * material.scale.y,
        );
    }

    // bottom-left corner
    if bottom && left {
        return vec2<f32>(
            uv.x * material.scale.x,
            1.0 - (1.0 - uv.y) * material.scale.y
        );
    }

    // bottom-right corner
    if bottom && right {
        return vec2<f32>(1.0) - (vec2<f32>(1.0) - uv) * material.scale;
    }

    // top edge
    if top {
        return vec2<f32>(
            tile_or_stretch(uv.x, min_inset_scaled.x, 1.0 - max_inset_scaled.x, material.min_inset.x, 1.0 - material.max_inset.x, material.side_stretch_value.x),
            uv.y * material.scale.y
        );
    }

    // bottom edge
    if bottom {
        return vec2<f32>(
            tile_or_stretch(uv.x, min_inset_scaled.x, 1.0 - max_inset_scaled.x, material.min_inset.x, 1.0 - material.max_inset.x, material.side_stretch_value.x),
            1.0 - (1.0 - uv.y) * material.scale.y
        );
    }

    // left edge
    if left {
        return vec2<f32>(
            uv.x * material.scale.x,
            tile_or_stretch(uv.y, min_inset_scaled.y, 1.0 - max_inset_scaled.y, material.min_inset.y, 1.0 - material.max_inset.y, material.side_stretch_value.y)
        );
    }

    // right edge
    if right {
        return vec2<f32>(
            1.0 - (1.0 - uv.x) * material.scale.x,
            tile_or_stretch(uv.y, min_inset_scaled.y, 1.0 - max_inset_scaled.y, material.min_inset.y, 1.0 - material.max_inset.y, material.side_stretch_value.y)
        );
    }

    // center
    return vec2<f32>(
        tile_or_stretch(uv.x, min_inset_scaled.x, 1.0 - max_inset_scaled.x, material.min_inset.x, 1.0 - material.max_inset.x, material.center_stretch_value.x),
        tile_or_stretch(uv.y, min_inset_scaled.y, 1.0 - max_inset_scaled.y, material.min_inset.y, 1.0 - material.max_inset.y, material.center_stretch_value.y)
    );
}

// Applies the tint from the sprite's `color` property.
fn apply_tint(sprite_color: vec4<f32>) -> vec4<f32> {
    return sprite_color * material.color;
}

// Discards fragments based on the sprite's `alpha_cutoff` and `alpha_mode`.
fn alpha_discard(output_color: vec4<f32>) -> vec4<f32> {
    var color = output_color;
    let alpha_mode = material.flags & SPRITE_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;

    if alpha_mode == SPRITE_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE {
        // NOTE: If rendering as opaque, alpha should be ignored so set to 1.0
        color.a = 1.0;
    }

#ifdef MAY_DISCARD
    else if alpha_mode == SPRITE_MATERIAL_FLAGS_ALPHA_MODE_MASK {
        if color.a >= material.alpha_cutoff {
            // NOTE: If rendering as masked alpha and >= the cutoff, render as fully opaque
            color.a = 1.0;
        } else {
            // NOTE: output_color.a < in.material.alpha_cutoff should not be rendered
            discard;
        }
    }
#endif // MAY_DISCARD

    return color;
}

// Maps a point p from [a, b] to [c, d], tiling it if stretch_value is not 0.
fn tile_or_stretch(p: f32, a: f32, b: f32, c: f32, d: f32, stretch_value: f32) -> f32 {
    if stretch_value == 0.0 {
        return stretch_interval(p, a, b, c, d);
    }
    return tile_interval(p, a, b, c, d, stretch_value);
}

// Takes a point p from an interval [a, b] and maps it to a portion of the tile [c, d]
fn tile_interval(p: f32, a: f32, b: f32, c: f32, d: f32, stretch_value: f32) -> f32 {
    let value = (p - a) / (b - a);
    let tile_value = (value - stretch_value * floor(value / stretch_value)) / stretch_value;
    return tile_value * (d - c) + c;
}

// Takes a point p from an interval [a, b] and translates it to the interval [c, d]
fn stretch_interval(p: f32, a: f32, b: f32, c: f32, d: f32) -> f32 {
    return (p - a) / (b - a) * (d - c) + c;
}
