# Zero-Day Example

Beeple's "Zero-Day" sci-fi corridor (NVIDIA ORCA), **path-traced with Bevy Solari**.

Zero-Day has no punctual lights. All of its light comes from approximately 10,000 emissive
triangles, as in NVIDIA's original real-time
["Measure 1"](https://www.youtube.com/watch?v=0WE7CgJMuVc) demo. This example needs Solari,
because only a path tracer can light the scene this way. Solari makes the emissive meshes
into area lights that give global illumination. The example plays the animation of the film
(approximately 550 objects and the camera flythrough), and the render camera follows the
film camera.

The lights in the film also pulse as the camera moves through the corridor, but Octane made
that sequence procedurally. It is **not** in the FBX of any ORCA measure, where all of the
animation is rigid transforms, and Bevy cannot import animated material properties through
glTF. `animate_emissive` therefore makes a substitute: a wave of light that moves along the
panels of the corridor. Use `--no-pulse` to disable it.

## Getting the scene

Download "Zero-Day" [from developer.nvidia.com](https://developer.nvidia.com/orca/beeple-zero-day).
The real-time demo videos from NVIDIA show how each measure must look:
[Measure 1](https://www.youtube.com/watch?v=0WE7CgJMuVc) and
[Measure 7](https://www.youtube.com/watch?v=zaOR22Q0RPc), which is the video on that page.

The download contains several "measures". Each measure is an `.fbx` file with a `tex/`
folder of `.dds` textures adjacent to it. This example can load any of them (see `--scene`
below):

| `--scene`                      | FBX                                              |
|:-------------------------------|:-------------------------------------------------|
| `measure_one` (default)        | `MEASURE_ONE/MEASURE_ONE.fbx`                    |
| `measure_seven`                | `MEASURE_SEVEN/MEASURE_SEVEN.fbx`                |
| `measure_seven_colored_lights` | `MEASURE_SEVEN/MEASURE_SEVEN_COLORED_LIGHTS.fbx` |

Bevy cannot load FBX, and the FBX importer of Blender reads the material conventions of
this Octane export incorrectly. [`convert.py`](convert.py) therefore builds each material
again from the name and channel convention that the README of the download specifies, bakes
the animation over the full scene frame range, and exports one self-contained `.glb`:

| Texture          | Channels                                                                        |
|:-----------------|:--------------------------------------------------------------------------------|
| `_BaseColor.dds` | RGB = base color (the alpha is not used, all surfaces are opaque)               |
| `_Specular.dds`  | R = occlusion, **G = roughness, B = metallic**                                  |
| `_Normal.dds`    | DirectX normal (the green channel is inverted to the OpenGL convention of glTF) |
| `_Emissive.dds`  | RGB = emissive color                                                            |

The script also prepares the meshes for Solari, which traces a mesh only if its vertex
layout is POSITION, NORMAL, UV0, and TANGENT. Each mesh gets one UV set (an empty set if it
had none) and baked tangents. The example then only has to change 16-bit indices to 32-bit
indices, which the glTF exporter cannot write.

The script also deletes the meshes that the FBX marks as hidden: proxy shells and ON and
OFF variants of the light states, approximately 1,700 objects in Measure Seven. The film
does not render them, but the glTF export would keep them as solid, visible geometry that
encloses both the camera and the rays of Solari.

Convert the scene with the headless Blender helper (Blender 4.x or 5.x) and put the result
in the `assets/` folder of this example. Git ignores that folder, and its contents are never
committed. Run the helper one time for each measure that you want. The example loads the
filenames below:

```console
# measure_one (the default)
blender --background --python-exit-code 1 --python convert.py -- \
  "MEASURE_ONE/MEASURE_ONE.fbx" \
  "examples/large_scenes/zero_day/assets/zero_day_measure_one.glb"

# measure_seven
blender --background --python-exit-code 1 --python convert.py -- \
  "MEASURE_SEVEN/MEASURE_SEVEN.fbx" \
  "examples/large_scenes/zero_day/assets/zero_day_measure_seven.glb"

# measure_seven_colored_lights (the geometry and the animation of measure_seven, with
# different emissive colors)
blender --background --python-exit-code 1 --python convert.py -- \
  "MEASURE_SEVEN/MEASURE_SEVEN_COLORED_LIGHTS.fbx" \
  "examples/large_scenes/zero_day/assets/zero_day_measure_seven_colored_lights.glb"
```

## Running

```console
cargo run -p zero_day --release
# a different measure (convert it first, see above):
cargo run -p zero_day --release -- --scene measure_seven_colored_lights
# with DLSS Ray Reconstruction (needs an NVIDIA RTX GPU and the DLSS SDK):
cargo run -p zero_day --release --features dlss
# more frames per second on the heavy measures, with less sharpness:
cargo run -p zero_day --release --features dlss -- --scene measure_seven --dlss-quality performance
```

DLSS is off by default, because a build with it needs the DLSS SDK, and this crate is part
of the workspace `cargo check`.

Controls:

- **C** - change between the film flythrough and free-fly (WASD and mouse).
- **N** - turn DLSS Ray Reconstruction on and off (with the `dlss` feature).
- **B** - do a short benchmark over the flythrough and print the result to the console.

```console
Options:
  --scene      which ORCA measure to load: measure_one (default), measure_seven, or
               measure_seven_colored_lights. convert.py makes a different .glb for each.
  --emissive   emissive multiplier for the accent panels (default 150000). The panels are
               the only lights in the scene, and they must be bright to light the space.
  --no-pulse   disable the synthetic emissive pulse. By default, a wave of light moves
               along the panels as a substitute for the animated lights of the film, which
               are not in the exported asset.
  --merge-animations
               combine the thousands of per-object clips of the glTF into one clip at
               startup. The playback is identical. The load is slower, but the animation
               evaluation in each frame is much less expensive.
  --no-solari  skip Solari and use a flat ambient light. The scene does not render
               correctly this way, because the panels that Solari resolves are its only
               real lights. This is an escape hatch for profiling and smoke tests, not a
               lighting mode. It runs on GPUs that cannot do ray tracing.
  --resolution render resolution as WxH (default 1920x1080). The Solari cost increases
               with the pixel count. Use a lower value (for example 1280x720) on the heavy
               measures to get more frames per second, with less sharpness.
  --dlss-quality  DLSS quality mode: auto (default), dlaa, quality, balanced, performance,
               or ultra_performance. A lower mode renders at a smaller internal resolution
               and gives more frames per second.
  --help       display usage information
```
