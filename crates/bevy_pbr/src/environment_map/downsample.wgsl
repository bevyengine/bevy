// Original source: https://www.activision.com/cdn/research/downsample_cubemap.txt

// Copyright 2016 Activision Publishing, Inc.
//
// Permission is hereby granted, free of charge, to any person obtaining
// a copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

@group(0) @binding(0) var tex_hi_re: texture_cube<f32>;
@group(0) @binding(1) var tex_los_res: texture_storage_2d_array<rg11b10float, write>;
@group(0) @binding(2) var bilinear: sampler;

fn get_dir(u: f32, v: f32, face: u32) -> vec3<f32> {
    switch face {
        case 0u: { return vec3(1.0, v, -u); }
        case 1u: { return vec3(-1.0, v, u); }
        case 2u: { return vec3(u, 1.0, -v); }
        case 3u: { return vec3(u, -1.0, v); }
        case 4u: { return vec3(u, v, 1.0); }
        default { return vec3(-u, v, -1.0); }
    }
}

fn calc_weight(u: f32, v: f32) -> f32 {
	let val = u * u + v * v + 1.0;
	return val * sqrt(val);
}

@compute
@workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let res_lo = textureDimensions(tex_los_res).x;

    if all(vec2u(id.xy) < vec2u(res_lo)) {
        let inv_res_lo = 1.0 / f32(res_lo);

        let u0 = (f32(id.x) * 2.0 + 1.0 - 0.75) * inv_res_lo - 1.0;
		let u1 = (f32(id.x) * 2.0 + 1.0 + 0.75) * inv_res_lo - 1.0;

		let v0 = (f32(id.y) * 2.0 + 1.0 - 0.75) * -inv_res_lo + 1.0;
		let v1 = (f32(id.y) * 2.0 + 1.0 + 0.75) * -inv_res_lo + 1.0;

        var weights = array(calc_weight(u0, v0), calc_weight(u1, v0), calc_weight(u0, v1), calc_weight(u1, v1));
        let wsum = 0.5 / (weights[0] + weights[1] + weights[2] + weights[3]);
        for (var i = 0u; i < 4u; i++) {
            weights[i] = weights[i] * wsum + 0.125;
        }

        var color = textureSampleLevel(tex_hi_re, bilinear, get_dir(u0, v0, id.z), 0.0) * weights[0];
        color += textureSampleLevel(tex_hi_re, bilinear, get_dir(u1, v0, id.z), 0.0) * weights[1];
        color += textureSampleLevel(tex_hi_re, bilinear, get_dir(u0, v1, id.z), 0.0) * weights[2];
        color += textureSampleLevel(tex_hi_re, bilinear, get_dir(u1, v1, id.z), 0.0) * weights[3];

        textureStore(tex_los_res, id.xy, id.z, color);
    }
}
