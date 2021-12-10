# Cargo Features

## Default Features

|feature name|description|
|-|-|
|bevy_audio|Audio support. Support for all audio formats depends on this.|
|bevy_gilrs|Adds gamepad support.|
|bevy_gltf|[glTF](https://www.khronos.org/gltf/) support.|
|bevy_winit|GUI support.|
|render|The render pipeline and all render related plugins.|
|png|PNG picture format support.|
|hdr|[HDR](https://en.wikipedia.org/wiki/High_dynamic_range) support.|
|mp3|MP3 audio format support.|
|x11|Make GUI applications use X11 protocol. You could enable wayland feature to override this.|
|filesystem_watcher|Enable watching the file system for asset hot reload|

## Optional Features

|feature name|description|
|-|-|
|bevy_dynamic_plugin|Plugin for dynamic loading (using [libloading](https://crates.io/crates/libloading)).|
|dynamic|Forces bevy to be dynamically linked, which improves iterative compile times.|
|trace|Enables system tracing.|
|trace_chrome|Enables [tracing-chrome](https://github.com/thoren-d/tracing-chrome) as bevy_log output. This allows you to visualize system execution.|
|trace_tracy|Enables [Tracy](https://github.com/wolfpld/tracy) as bevy_log output. This allows `Tracy` to connect to and capture profiling data as well as visualize system execution in real-time, present statistics about system execution times, and more.|
|wgpu_trace|For tracing wgpu.|
|dds|DDS picture format support.|
|tga|TGA picture format support.|
|jpeg|JPEG picture format support.|
|bmp|BMP picture format support.|
|flac|FLAC audio format support. It's included in bevy_audio feature.|
|wav|WAV audio format support.|
|vorbis|Vorbis audio format support.|
|serialize|Enables serialization of `bevy_input` types.|
|wayland|Enable this to use Wayland display server protocol other than X11.|
|webgl2|Enable this to use WebGL2 optimised code.|
|subpixel_glyph_atlas|Enable this to cache glyphs using subpixel accuracy. This increases texture memory usage as each position requires a separate sprite in the glyph atlas, but provide more accurate character spacing.|
|bevy_ci_testing|Used for running examples in CI.|
