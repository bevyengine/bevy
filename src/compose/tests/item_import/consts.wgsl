#define_import_path consts

let X: u32 = 1u;
let Y: u32 = 2u;
let Z: u32 = 3u;

@group(0) @binding(0)
var something: sampler;

fn double(in: u32) -> u32 {
    return in * 2u;
}

fn triple(in: u32) -> u32 {
    return in * 3u;
}