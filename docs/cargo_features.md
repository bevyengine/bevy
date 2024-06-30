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
|bevy_color|Provides shared color types and operations|
|bevy_core_pipeline|Provides cameras and other basic render pipeline features|
|bevy_gilrs|Adds gamepad support|
|bevy_gizmos|Adds support for rendering gizmos|
|bevy_gltf|[glTF](https://www.khronos.org/gltf/) support|
|bevy_pbr|Adds PBR rendering|
|bevy_picking|Provides picking functionality|
|bevy_render|Provides rendering functionality|
|bevy_scene|Provides scene functionality|
|bevy_sprite|Provides sprite functionality|
|bevy_state|Enable built in global state machines|
|bevy_text|Provides text functionality|
|bevy_ui|A custom ECS-driven UI framework|
|bevy_winit|winit window and input backend|
|default_font|Include a default font, containing only ASCII characters, at the cost of a 20kB binary size increase|
|hdr|HDR image format support|
|ktx2|KTX2 compressed texture support|
|multi_threaded|Enables multithreaded parallelism in the engine. Disabling it forces all engine tasks to run on a single thread.|
|png|PNG image format support|
|sysinfo_plugin|Enables system information diagnostic plugin|
|tonemapping_luts|Include tonemapping Look Up Tables KTX2 files. If everything is pink, you need to enable this feature or change the `Tonemapping` method on your `Camera2dBundle` or `Camera3dBundle`.|
|vorbis|OGG/VORBIS audio format support|
|webgl2|Enable some limitations to be able to use WebGL2. Please refer to the [WebGL2 and WebGPU](https://github.com/bevyengine/bevy/tree/latest/examples#webgl2-and-webgpu) section of the examples README for more information on how to run Wasm builds with WebGPU.|
|x11|X11 display server support|
|zstd|For KTX2 supercompression|

### Optional Features

|feature name|description|
|-|-|
|accesskit_unix|Enable AccessKit on Unix backends (currently only works with experimental screen readers and forks.)|
|asset_processor|Enables the built-in asset processor for processed assets.|
|async-io|Use async-io's implementation of block_on instead of futures-lite's implementation. This is preferred if your application uses async-io.|
|basis-universal|Basis Universal compressed texture support|
|bevy_ci_testing|Enable systems that allow for automated testing on CI|
|bevy_debug_stepping|Enable stepping-based debugging of Bevy systems|
|bevy_dev_tools|Provides a collection of developer tools|
|bevy_dynamic_plugin|Plugin for dynamic loading (using [libloading](https://crates.io/crates/libloading))|
|bmp|BMP image format support|
|dds|DDS compressed texture support|
|debug_glam_assert|Enable assertions in debug builds to check the validity of parameters passed to glam|
|detailed_trace|Enable detailed trace event logging. These trace events are expensive even when off, thus they require compile time opt-in|
|dynamic_linking|Force dynamic linking, which improves iterative compile times|
|embedded_watcher|Enables watching in memory asset providers for Bevy Asset hot-reloading|
|exr|EXR image format support|
|file_watcher|Enables watching the filesystem for Bevy Asset hot-reloading|
|flac|FLAC audio format support|
|glam_assert|Enable assertions to check the validity of parameters passed to glam|
|ios_simulator|Enable support for the ios_simulator by downgrading some rendering capabilities|
|jpeg|JPEG image format support|
|meshlet|Enables the meshlet renderer for dense high-poly scenes (experimental)|
|meshlet_processor|Enables processing meshes into meshlet meshes for bevy_pbr|
|minimp3|MP3 audio format support (through minimp3)|
|mp3|MP3 audio format support|
|pbr_multi_layer_material_textures|Enable support for multi-layer material textures in the `StandardMaterial`, at the risk of blowing past the global, per-shader texture limit on older/lower-end GPUs|
|pbr_transmission_textures|Enable support for transmission-related textures in the `StandardMaterial`, at the risk of blowing past the global, per-shader texture limit on older/lower-end GPUs|
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
|webgpu|Enable support for WebGPU in Wasm. When enabled, this feature will override the `webgl2` feature and you won't be able to run Wasm builds with WebGL2, only with WebGPU.|
|webp|WebP image format support|
|wgpu_trace|Save a trace of all wgpu calls|
|zlib|For KTX2 supercompression|
