# Pathfinder 3

![Logo](https://github.com/servo/pathfinder/raw/master/resources/textures/pathfinder-logo.png)

Pathfinder 3 is a fast, practical, GPU-based rasterizer for fonts and vector graphics using OpenGL
3.0+, OpenGL ES 3.0+, WebGL 2, and Metal.

Please note that Pathfinder is under heavy development and is incomplete in various areas.

## Quick start

Pathfinder contains a library that implements a subset of the
[HTML canvas API](https://developer.mozilla.org/en-US/docs/Web/API/Canvas_API). You can quickly add
vector rendering to any Rust app with it. The library is available on `crates.io`. See
`examples/canvas_minimal` for a small example of usage.

### Demos

Demo app sources are available in [demo/](https://github.com/servo/pathfinder/tree/master/demo). A prebuilt package for Magic Leap can be found in [releases](https://github.com/servo/pathfinder/releases).

## Features

The project features:

* High quality antialiasing. Pathfinder can compute exact fractional trapezoidal area coverage on a
  per-pixel basis for the highest-quality antialiasing possible (effectively 256xAA).

* Fast CPU setup, making full use of parallelism. Pathfinder 3 uses the Rayon library to quickly
  perform a CPU tiling prepass to prepare vector scenes for the GPU. This prepass can be pipelined
  with the GPU to hide its latency.

* Fast GPU rendering, even at small pixel sizes. Even on lower-end GPUs, Pathfinder typically
  matches or exceeds the performance of the best CPU rasterizers. The difference is particularly
  pronounced at large sizes, where Pathfinder regularly achieves multi-factor speedups.

* GPU compute-based rendering, where available. Pathfinder can optionally use compute shaders to
  achieve better performance than what the built-in GPU rasterization hardware can provide. Compute
  shader capability is not required, and all features are available without it.

* Advanced font rendering. Pathfinder can render fonts with slight hinting and can perform subpixel
  antialiasing on LCD screens. It can do stem darkening/font dilation like macOS and FreeType in
  order to make text easier to read at small sizes. The library also has support for gamma
  correction.

* Support for SVG. Pathfinder 3 is designed to efficiently handle workloads that consist of many
  overlapping vector paths, such as those commonly found in SVG and PDF files. It can perform
  occlusion culling, which often results in dramatic performance wins over typical software
  renderers that use the painter's algorithm. A simple loader that leverages the `resvg` library
  to render a subset of SVG is included, so it's easy to get started.

* 3D capability. Pathfinder can render fonts and vector paths in 3D environments without any loss
  in quality. This is intended to be useful for vector-graphics-based user interfaces in VR, for
  example.

* Lightweight. Unlike large vector graphics packages that mix and match many different algorithms,
  Pathfinder 3 uses a single, simple technique. It consists of a set of modular crates, so
  applications can pick and choose only the components that are necessary to minimize dependencies.

* Portability to most GPUs manufactured in the last decade, including integrated and mobile GPUs.
  Geometry, tessellation, and compute shader functionality is not required.

## Building

Pathfinder 3 is a set of modular packages, allowing you to choose which parts of the library you
need. An SVG rendering demo, written in Rust, is included, so you can try Pathfinder out right
away. It also provides an example of how to use the library. (Note that, like the rest of
Pathfinder, the demo is under heavy development and has known bugs.)

Running the demo is as simple as:

    $ cd demo/native
    $ cargo run --release

Running examples (e.g. `canvas_nanovg`) can be done with:

    $ cd examples/canvas_nanovg
    $ cargo run --release

Pathfinder libraries are available on `crates.io` with the `pathfinder_` prefix (e.g.
`pathfinder_canvas`), but you may wish to use the `master` branch for the latest features and bug
fixes.

## Community

There's a Matrix chat room available at
[`#pathfinder:mozilla.org`](https://matrix.to/#/!XiDASQfNTTMrJbXHTw:mozilla.org?via=mozilla.org).
If you're on the Mozilla Matrix server, you can search for Pathfinder to find it. For more
information on connecting to the Matrix network, see
[this `wiki.mozilla.org` page](https://wiki.mozilla.org/Matrix).

The entire Pathfinder community, including the chat room and GitHub project, is expected to abide
by the same Code of Conduct that the Rust project itself follows.

## Build status

[![Build Status](https://travis-ci.org/servo/pathfinder.svg?branch=master)](https://travis-ci.org/servo/pathfinder)

## Authors

The primary author is Patrick Walton (@pcwalton), with contributions from the Servo development
community.

The logo was designed by Jay Vining.

## License

Pathfinder is licensed under the same terms as Rust itself. See `LICENSE-APACHE` and `LICENSE-MIT`.

Material Design icons are copyright Google Inc. and licensed under the Apache 2.0 license.
