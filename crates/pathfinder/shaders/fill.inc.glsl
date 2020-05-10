// pathfinder/shaders/fill.inc.glsl
//
// Copyright © 2020 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

vec4 computeCoverage(vec2 from, vec2 to, texture2D areaLUT, sampler areaLUTSampler) {
    // Determine winding, and sort into a consistent order so we only need to find one root below.
    vec2 left = from.x < to.x ? from : to, right = from.x < to.x ? to : from;

    // Shoot a vertical ray toward the curve.
    vec2 window = clamp(vec2(from.x, to.x), -0.5, 0.5);
    float offset = mix(window.x, window.y, 0.5) - left.x;
    float t = offset / (right.x - left.x);

    // Compute position and derivative to form a line approximation.
    float y = mix(left.y, right.y, t);
    float d = (right.y - left.y) / (right.x - left.x);

    // Look up area under that line, and scale horizontally to the window size.
    float dX = window.x - window.y;
    return texture(sampler2D(areaLUT, areaLUTSampler), vec2(y + 8.0, abs(d * dX)) / 16.0) * dX;
}
