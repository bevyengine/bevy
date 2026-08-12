# Zero-Day Example

Beeple's "Zero-Day" sci-fi corridor (NVIDIA ORCA), path-traced with Bevy Solari. All of
the light comes from approximately 10,000 emissive triangles, which Solari turns into area
lights. The example plays the film's animation and follows the film camera.

## Getting the scene

Download "Zero-Day" [from developer.nvidia.com](https://developer.nvidia.com/orca/beeple-zero-day).

Bevy can't load FBX assets, so convert each measure that you want with the headless Blender
helper (Blender 4.x or 5.x) and put the result in this example's `assets/` folder:

```console
# measure_one (the default)
blender --background --python-exit-code 1 --python convert.py -- \
  "MEASURE_ONE/MEASURE_ONE.fbx" \
  "examples/large_scenes/zero_day/assets/zero_day_measure_one.glb"

# measure_seven
blender --background --python-exit-code 1 --python convert.py -- \
  "MEASURE_SEVEN/MEASURE_SEVEN.fbx" \
  "examples/large_scenes/zero_day/assets/zero_day_measure_seven.glb"

# measure_seven_colored_lights
blender --background --python-exit-code 1 --python convert.py -- \
  "MEASURE_SEVEN/MEASURE_SEVEN_COLORED_LIGHTS.fbx" \
  "examples/large_scenes/zero_day/assets/zero_day_measure_seven_colored_lights.glb"
```

## Running

```console
cargo run -p zero_day --release
cargo run -p zero_day --release -- --scene measure_seven
# with DLSS Ray Reconstruction (needs an NVIDIA RTX GPU and the DLSS SDK):
cargo run -p zero_day --release --features dlss
```

Press C to change between the film flythrough and free-fly (WASD and mouse). Press N to
turn DLSS Ray Reconstruction on and off. Press B for benchmark (see console for results).

```console
Options:
  --scene       which scene to load: measure_one (default), measure_seven, or
                measure_seven_colored_lights
  --emissive    emissive multiplier for the light panels (default 150000)
  --no-pulse    disable the synthetic emissive pulse that substitutes for the film's
                animated lights
  --no-solari   render without Solari, with a flat ambient light instead (not
                representative; for profiling and smoke tests)
  --resolution  render resolution as WxH (default 1920x1080)
  --dlss-quality
                DLSS quality mode: auto (default), dlaa, quality, balanced, performance,
                or ultra_performance
  --help        display usage information
```
