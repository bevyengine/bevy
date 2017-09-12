# mikktspace

[![crates.io](https://img.shields.io/crates/v/mikktspace.svg)](https://crates.io/crates/mikktspace)
[![Build Status](https://travis-ci.org/gltf-rs/mikktspace.svg?branch=master)](https://travis-ci.org/gltf-rs/mikktspace)

Bindings to the [Mikkelsen Tangent Space Algorithm](https://wiki.blender.org/index.php/Dev:Shading/Tangent_Space_Normal_Maps) reference implementation.

## Examples

### generate

Demonstrates generating tangents for a cube with 4 triangular faces per side.

```sh
cargo run --example generate
```

There is also an equivalent C example to check the correctness of the Rust bindings.

```sh
cd examples
cmake ../libmikktspace
make
cc generate.c libmikktspace.a -I../libmikktspace -lm -o generate
./generate
```

## License agreement

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
