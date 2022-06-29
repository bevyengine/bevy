# bevy_mikktspace

This is a fork of [https://github.com/gltf-rs/mikktspace](https://github.com/gltf-rs/mikktspace), which in turn is a port of the Mikkelsen Tangent Space Algorithm reference implementation to Rust. It has been forked for use in the bevy game engine to be able to update maths crate dependencies in lock-step with bevy releases. It is vendored in the bevy repository itself as [crates/bevy_mikktspace](https://github.com/bevyengine/bevy/tree/main/crates/bevy_mikktspace).

Port of the [Mikkelsen Tangent Space Algorithm](https://en.blender.org/index.php/Dev:Shading/Tangent_Space_Normal_Maps) reference implementation.

Requires at least Rust 1.52.1.

## Examples

### generate

Demonstrates generating tangents for a cube with 4 triangular faces per side.

```sh
cargo run --example generate
```

## License agreement

Licensed under either of

* Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))
* MIT license
  ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))

at your option. AND parts of the code are licensed under:

* Zlib license
  [https://opensource.org/licenses/Zlib](https://opensource.org/licenses/Zlib)

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
