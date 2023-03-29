// Copyright (c) 2022 Advanced Micro Devices, Inc. All rights reserved.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

#import bevy_core_pipeline::fullscreen_vertex_shader

struct CASUniforms {
    sharpness: f32,
};

@group(0) @binding(0)
var screenTexture: texture_2d<f32>;
@group(0) @binding(1)
var samp: sampler;
@group(0) @binding(2)
var<uniform> uniforms: CASUniforms;

// This is set at the limit of providing unnatural results for sharpening.
const FSR_RCAS_LIMIT = 0.1875;
// -4.0 instead of -1.0 to avoid issues with MSAA.
const peakC = vec2<f32>(10.0, -40.0);

// Robust Contrast Adaptive Sharpening (RCAS)
// RCAS is based on the following logic.
// RCAS uses a 5 tap filter in a cross pattern (same as CAS),
//    W                b
//  W 1 W  for taps  d e f 
//    W                h
// Where 'W' is the negative lobe weight.
//  output = (W*(b+d+f+h)+e)/(4*W+1)
// RCAS solves for 'W' by seeing where the signal might clip out of the {0 to 1} input range,
//  0 == (W*(b+d+f+h)+e)/(4*W+1) -> W = -e/(b+d+f+h)
//  1 == (W*(b+d+f+h)+e)/(4*W+1) -> W = (1-e)/(b+d+f+h-4)
// Then chooses the 'W' which results in no clipping, limits 'W', and multiplies by the 'sharp' amount.
// This solution above has issues with MSAA input as the steps along the gradient cause edge detection issues.
// So RCAS uses 4x the maximum and 4x the minimum (depending on equation)in place of the individual taps.
// As well as switching from 'e' to either the minimum or maximum (depending on side), to help in energy conservation.
// This stabilizes RCAS.
// RCAS does a simple highpass which is normalized against the local contrast then shaped,
//       0.25
//  0.25  -1  0.25
//       0.25
// This is used as a noise detection filter, to reduce the effect of RCAS on grain, and focus on real edges.
// The CAS node runs after tonemapping, so the input should be in the range of 0 to 1.
@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
     // Algorithm uses minimal 3x3 pixel neighborhood.
     //    b
     //  d e f
     //    h
	let b = textureSample(screenTexture, samp, in.uv, vec2<i32>(0, -1)).rgb;
	let d = textureSample(screenTexture, samp, in.uv, vec2<i32>(-1, 0)).rgb;
	// We need the alpha value of the pixel we're working on for the output
	let e = textureSample(screenTexture, samp, in.uv).rgbw;
	let f = textureSample(screenTexture, samp, in.uv, vec2<i32>(1, 0)).rgb;
	let h = textureSample(screenTexture, samp, in.uv, vec2<i32>(0, 1)).rgb;
    // Min and max of ring.
    let mn4 = min(min(b, d), min(f, h));
    let mx4 = max(max(b, d), max(f, h));
    // Limiters
    // 4.0 to avoid issues with MSAA.
    let hitMin = mn4 / (4.0 * mx4);
    let hitMax = (peakC.x - mx4) / (peakC.y + 4.0 * mn4);
    let lobeRGB = max(-hitMin, hitMax);
    var lobe = max(-FSR_RCAS_LIMIT, min(0.0, max(lobeRGB.r, max(lobeRGB.g, lobeRGB.b)))) * uniforms.sharpness;
#ifdef RCAS_DENOISE
    // Luma times 2.
    let bL = b.g + 0.5 * (b.b + b.r);
    let dL = d.g + 0.5 * (d.b + d.r);
    let eL = e.g + 0.5 * (e.b + e.r);
    let fL = f.g + 0.5 * (f.b + f.r);
    let hL = h.g + 0.5 * (h.b + h.r);
    // Noise detection.
    var noise = 0.25 * (bL + dL + fL + hL) - eL;
    noise = saturate(abs(noise) / (max(max(bL, dL), max(fL, hL)) - min(min(bL, dL), min(fL, hL))));
    noise = 1.0 - 0.5 * noise;
    // Apply noise removal.
    lobe *= noise;
#endif
    return vec4<f32>((lobe * (b + d + f + h) + e.rgb) / (4.0 * lobe + 1.0), e.w);
}
