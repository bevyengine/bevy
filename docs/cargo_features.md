<!-- MD041 - This file will be included in docs and should not start with a top header -->
<!-- markdownlint-disable-file MD041 -->

## Cargo Features

Bevy exposes many features to customise the engine. Enabling them add functionalities but often come at the cost of longer compilation times and extra dependencies.

### Default Features

The default feature set enables most of the expected features of a game engine, like rendering in both 2D and 3D, asset loading, audio and UI. To help reduce compilation time, consider disabling default features and enabling only those you need.

|feature name|description|
|-|-|
|android_shared_stdcxx|Enable using a shared stdlib for cxx on Android|
|animation|Enable animation support, and glTF animation loading|
|bevy_animation|Provides animation functionality|
|bevy_asset|Provides asset functionality|
|bevy_audio|Provides audio functionality|
|bevy_core_pipeline|Provides cameras and other basic render pipeline features|
|bevy_gilrs|Adds gamepad support|
|bevy_gizmos|Adds support for rendering gizmos|
|bevy_gltf|[glTF](https://www.khronos.org/gltf/) support|
|bevy_pbr|Adds PBR rendering|
|bevy_render|Provides rendering functionality|
|bevy_scene|Provides scene functionality|
|bevy_sprite|Provides sprite functionality|
|bevy_text|Provides text functionality|
|bevy_ui|A custom ECS-driven UI framework|
|bevy_winit|winit window and input backend|
|default_font|Include a default font, containing only ASCII characters, at the cost of a 20kB binary size increase|
|hdr|HDR image format support|
|ktx2|KTX2 compressed texture support|
|multi-threaded|Enables multithreaded parallelism in the engine. Disabling it forces all engine tasks to run on a single thread.|
|png|PNG image format support|
|tonemapping_luts|Include tonemapping Look Up Tables KTX2 files|
|vorbis|OGG/VORBIS audio format support|
|webgl2|Enable some limitations to be able to use WebGL2. If not enabled, it will default to WebGPU in Wasm|
|x11|X11 display server support|
|zstd|For KTX2 supercompression|

### Optional Features

|feature name|description|
|-|-|
|accesskit_unix|Enable AccessKit on Unix backends (currently only works with experimental screen readers and forks.)|
|async-io|Use async-io's implementation of block_on instead of futures-lite's implementation. This is preferred if your application uses async-io.|
|basis-universal|Basis Universal compressed texture support|
|bevy_ci_testing|Enable systems that allow for automated testing on CI|
|bevy_dynamic_plugin|Plugin for dynamic loading (using [libloading](https://crates.io/crates/libloading))|
|bmp|BMP image format support|
|dds|DDS compressed texture support|
|detailed_trace|Enable detailed trace event logging. These trace events are expensive even when off, thus they require compile time opt-in|
|dynamic_linking|Force dynamic linking, which improves iterative compile times|
|exr|EXR image format support|
|filesystem_watcher|Enables watching the filesystem for Bevy Asset hot-reloading|
|flac|FLAC audio format support|
|glam_assert|Enable assertions to check the validity of parameters passed to glam|
|jpeg|JPEG image format support|
|minimp3|MP3 audio format support (through minimp3)|
|mp3|MP3 audio format support|
|pnm|PNM image format support, includes pam, pbm, pgm and ppm|
|serialize|Enable serialization support through serde|
|shader_format_glsl|Enable support for shaders in GLSL|
|shader_format_spirv|Enable support for shaders in SPIR-V|
|subpixel_glyph_atlas|Enable rendering of font glyphs using subpixel accuracy|
|symphonia-aac|AAC audio format support (through symphonia)|
|symphonia-all|AAC, FLAC, MP3, MP4, OGG/VORBIS, and WAV audio formats support (through symphonia)|
|symphonia-flac|FLAC audio format support (through symphonia)|
|symphonia-isomp4|MP4 audio format support (through symphonia)|
|symphonia-vorbis|OGG/VORBIS audio format support (through symphonia)|
|symphonia-wav|WAV audio format support (through symphonia)|
|tga|TGA image format support|
|trace|Tracing support|
|trace_chrome|Tracing support, saving a file in Chrome Tracing format|
|trace_tracy|Tracing support, exposing a port for Tracy|
|trace_tracy_memory|Tracing support, with memory profiling, exposing a port for Tracy|
|wav|WAV audio format support|
|wayland|Wayland display server support|
|webp|WebP image format support|
|wgpu_trace|Save a trace of all wgpu calls|
|zlib|For KTX2 supercompression|
