# Zero-Day Example

Beeple's "Zero-Day" sci-fi corridor from NVIDIA ORCA, path-traced with Bevy Solari. All of
the light comes from approximately 10,000 emissive triangles, which Solari turns into area
lights. The example plays the film's animation and follows the film camera.

## Getting the scene

Download the converted glTF binaries from the
[`zero-day-assets-v1` release](https://github.com/pavlov-net/bevy-examples/releases/tag/zero-day-assets-v1)
into this example's `assets/` folder. The example loads `measure_one` by default; the
other two measures are optional.

```console
curl -L -o examples/large_scenes/zero_day/assets/zero_day_measure_one.glb \
  https://github.com/pavlov-net/bevy-examples/releases/download/zero-day-assets-v1/zero_day_measure_one.glb

curl -L -o examples/large_scenes/zero_day/assets/zero_day_measure_seven.glb \
  https://github.com/pavlov-net/bevy-examples/releases/download/zero-day-assets-v1/zero_day_measure_seven.glb

curl -L -o examples/large_scenes/zero_day/assets/zero_day_measure_seven_colored_lights.glb \
  https://github.com/pavlov-net/bevy-examples/releases/download/zero-day-assets-v1/zero_day_measure_seven_colored_lights.glb
```

To convert the scene from source instead, download "Zero-Day"
[from developer.nvidia.com](https://developer.nvidia.com/orca/beeple-zero-day) and run the
[`convert.py`](https://github.com/pavlov-net/bevy-examples/blob/main/examples/zero_day/convert.py)
Blender script per the
[instructions in bevy-examples](https://github.com/pavlov-net/bevy-examples/tree/main/examples/zero_day#converting-from-source).

"Zero-Day" is by Mike Winkelmann (Beeple), licensed
[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/); the converted files are
modified from the original.

## Running

```console
cargo run -p zero_day --release
cargo run -p zero_day --release -- --scene measure_seven
# with DLSS Ray Reconstruction; needs an NVIDIA RTX GPU and the DLSS SDK
cargo run -p zero_day --release --features dlss
```

Press C to change between the film flythrough and free-fly with WASD and the mouse. Press
N to turn DLSS Ray Reconstruction on and off. Press B to run a short benchmark and print
the result to the console.

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
