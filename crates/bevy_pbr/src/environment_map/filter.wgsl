// Original source: https://www.activision.com/cdn/research/filter_using_table_128.txt

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

@group(0) @binding(0) var tex_in: texture_cube<f32>;
@group(0) @binding(1) var text_out0: texture_storage_2d_array<rg11b10float, write>;
@group(0) @binding(2) var text_out1: texture_storage_2d_array<rg11b10float, write>;
@group(0) @binding(3) var text_out2: texture_storage_2d_array<rg11b10float, write>;
@group(0) @binding(4) var text_out3: texture_storage_2d_array<rg11b10float, write>;
@group(0) @binding(5) var text_out4: texture_storage_2d_array<rg11b10float, write>;
@group(0) @binding(6) var text_out5: texture_storage_2d_array<rg11b10float, write>;
@group(0) @binding(7) var text_out6: texture_storage_2d_array<rg11b10float, write>;
@group(0) @binding(8) var trilinear: sampler;

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

@compute
@workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    var id = id;
    var level = 0u;
    if id.x < 128u * 128u {
        level = 0u;
    } else if id.x < 128u * 128u + 64u * 64u {
        level = 1u;
        id.x -= 128u * 128u;
    } else if id.x < 128u * 128u + 64u * 64u + 32u * 32u {
        level = 2u;
        id.x -= 128u * 128u + 64u * 64u;
    } else if id.x < 128u * 128u + 64u * 64u + 32u * 32u + 16u * 16u {
        level = 3u;
        id.x -= 128u * 128u + 64u * 64u + 32u * 32u;
    } else if id.x < 128u * 128u + 64u * 64u + 32u * 32u + 16u * 16u + 8u * 8u {
        level = 4u;
        id.x -= 128u * 128u + 64u * 64u + 32u * 32u + 16u * 16u;
    } else if id.x < 128u * 128u + 64u * 64u + 32u * 32u + 16u * 16u + 8u * 8u + 4u * 4u {
        level = 5u;
        id.x -= 128u * 128u + 64u * 64u + 32u * 32u + 16u * 16u + 8u * 8u;
    } else if id.x < 128u * 128u + 64u * 64u + 32u * 32u + 16u * 16u + 8u * 8u + 4u * 4u + 2u * 2u {
        level = 6u;
        id.x -= 128u * 128u + 64u * 64u + 32u * 32u + 16u * 16u + 8u * 8u + 4u * 4u;
    } else {
        return;
    }

    id.z = id.y;
    let res = 128u >> level;
    id.y = id.x / res;
    id.x -= id.y * res;

    let u = (f32(id.x) * 2.0 + 1.0) / f32(res) - 1.0;
    let v = -(f32(id.y) * 2.0 + 1.0) / f32(res) + 1.0;

    let dir = get_dir(u, v, id.z);
    let frame_z = normalize(dir);
    let adir = abs(dir);

    var color = vec4(0.0);
    for (var axis = 0u; axis < 3u; axis++) {
        let other_axis0 = 1u - (axis & 1u) - (axis >> 1u);
        let other_axis1 = 2u - (axis >> 1u);

        let frame_weight = (max(adir[other_axis0], adir[other_axis1]) - 0.75) / 0.25;
        if frame_weight > 0.0 {
            var up_vector = vec3(0.0);
            switch axis {
                case 0u: { up_vector = vec3(1.0, 0.0, 0.0); }
                case 1u: { up_vector = vec3(0.0, 1.0, 0.0); }
                default { up_vector = vec3(0.0, 0.0, 1.0); }
            }
            let frame_x = normalize(cross(up_vector, frame_z));
            let frame_y = cross(frame_z, frame_x);

            var nx = dir[other_axis0];
            var ny = dir[other_axis1];
            let nz = adir[axis];

            let nmax_xy = max(abs(ny), abs(nx));
            nx /= nmax_xy;
            ny /= nmax_xy;

            var theta = 0.0;
            if ny < nx {
                if ny <= -0.999 { theta = nx; } else { theta = ny; }
            } else {
                if ny >= 0.999 { theta = -nx; } else { theta = -ny; }
            }

            var phi = 0.0;
            if nz <= -0.999 {
                phi = -nmax_xy;
            } else if nz >= 0.999 {
                phi = nmax_xy;
            } else {
                phi = nz;
            }
            let theta2 = theta * theta;
            let phi2 = phi * phi;

            for (var i_super_tap = 0u; i_super_tap < 8u; i_super_tap++) {
                // TODO
            }
        }
    }
    color /= color.a;

    color.r = max(0.0, color.r);
    color.g = max(0.0, color.g);
    color.b = max(0.0, color.b);
    color.a = 1.0;

    switch level {
        case 0u: { textureStore(text_out0, id.xy, id.z, color); }
        case 1u: { textureStore(text_out1, id.xy, id.z, color); }
        case 2u: { textureStore(text_out2, id.xy, id.z, color); }
        case 3u: { textureStore(text_out3, id.xy, id.z, color); }
        case 4u: { textureStore(text_out4, id.xy, id.z, color); }
        case 5u: { textureStore(text_out5, id.xy, id.z, color); }
        default { textureStore(text_out6, id.xy, id.z, color); }
    }
}
