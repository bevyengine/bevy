<!-- MD041 - This file will be included in docs and should not start with a top header -->
<!-- markdownlint-disable-file MD041 -->

## Cargo Features

Bevy exposes many features to customize the engine. Enabling them add functionalities but often come at the cost of longer compilation times and extra dependencies.

### Default Features

The default feature set enables most of the expected features of a game engine, like rendering in both 2D and 3D, asset loading, audio and UI. To help reduce compilation time, consider disabling default features and enabling only those you need.

|feature name|description|
|-|-|
|android-game-activity|Android GameActivity support. Default, choose between this and `android-native-activity`.|
|android_shared_stdcxx|Enable using a shared stdlib for cxx on Android|
|animation|Enable animation support, and glTF animation loading|
|async_executor|Uses `async-executor` as a task execution backend.|
|bevy_animation|Provides animation functionality|
|bevy_anti_alias|Provides various anti aliasing solutions|
|bevy_asset|Provides asset functionality|
|bevy_audio|Provides audio functionality|
|bevy_camera|Provides camera and visibility types, as well as culling primitives.|
|bevy_color|Provides shared color types and operations|
|bevy_core_pipeline|Provides cameras and other basic render pipeline features|
|bevy_core_widgets|Headless widget collection for Bevy UI.|
|bevy_gilrs|Adds gamepad support|
|bevy_gizmos|Adds support for rendering gizmos|
|bevy_gltf|[glTF](https://www.khronos.org/gltf/) support|
|bevy_image|Load and access image data. Usually added by an image format|
|bevy_input_focus|Enable input focus subsystem|
|bevy_light|Provides light types such as point lights, directional lights, spotlights.|
|bevy_log|Enable integration with `tracing` and `log`|
|bevy_mesh|Provides a mesh format and some primitive meshing routines.|
|bevy_mesh_picking_backend|Provides an implementation for picking meshes|
|bevy_pbr|Adds PBR rendering|
|bevy_picking|Provides picking functionality|
|bevy_render|Provides rendering functionality|
|bevy_scene|Provides scene functionality|
|bevy_shader|Provides shaders usable through asset handles.|
|bevy_sprite|Provides sprite functionality|
|bevy_sprite_picking_backend|Provides an implementation for picking sprites|
|bevy_sprite_render|Provides sprite rendering functionality|
|bevy_state|Enable built in global state machines|
|bevy_text|Provides text functionality|
|bevy_ui|A custom ECS-driven UI framework|
|bevy_ui_picking_backend|Provides an implementation for picking UI|
|bevy_ui_render|Provides rendering functionality for bevy_ui|
|bevy_window|Windowing layer|
|bevy_winit|winit window and input backend|
|custom_cursor|Enable winit custom cursor support|
|debug|Enable collecting debug information about systems and components to help with diagnostics|
|default_font|Include a default font, containing only ASCII characters, at the cost of a 20kB binary size increase|
|hdr|HDR image format support|
|ktx2|KTX2 compressed texture support|
|multi_threaded|Enables multithreaded parallelism in the engine. Disabling it forces all engine tasks to run on a single thread.|
|png|PNG image format support|
|reflect_auto_register|Enable automatic reflect registration|
|smaa_luts|Include SMAA Look Up Tables KTX2 Files|
|std|Allows access to the `std` crate.|
|sysinfo_plugin|Enables system information diagnostic plugin|
|tonemapping_luts|Include tonemapping Look Up Tables KTX2 files. If everything is pink, you need to enable this feature or change the `Tonemapping` method for your `Camera2d` or `Camera3d`.|
|vorbis|OGG/VORBIS audio format support|
|wayland|Wayland display server support|
|webgl2|Enable some limitations to be able to use WebGL2. Please refer to the [WebGL2 and WebGPU](https://github.com/bevyengine/bevy/tree/latest/examples#webgl2-and-webgpu) section of the examples README for more information on how to run Wasm builds with WebGPU.|
|x11|X11 display server support|
|zstd_rust|For KTX2 Zstandard decompression using pure rust [ruzstd](https://crates.io/crates/ruzstd). This is the safe default. For maximum performance, use "zstd_c".|

### Optional Features

|feature name|description|
|-|-|
|accesskit_unix|Enable AccessKit on Unix backends (currently only works with experimental screen readers and forks.)|
|android-native-activity|Android NativeActivity support. Legacy, should be avoided for most new Android games.|
|asset_processor|Enables the built-in asset processor for processed assets.|
|async-io|Use async-io's implementation of block_on instead of futures-lite's implementation. This is preferred if your application uses async-io.|
|basis-universal|Basis Universal compressed texture support|
|bevy_ci_testing|Enable systems that allow for automated testing on CI|
|bevy_debug_stepping|Enable stepping-based debugging of Bevy systems|
|bevy_dev_tools|Provides a collection of developer tools|
|bevy_remote|Enable the Bevy Remote Protocol|
|bevy_solari|Provides raytraced lighting (experimental)|
|bevy_ui_debug|Provides a debug overlay for bevy UI|
|bluenoise_texture|Include spatio-temporal blue noise KTX2 file used by generated environment maps, Solari and atmosphere|
|bmp|BMP image format support|
|compressed_image_saver|Enables compressed KTX2 UASTC texture output on the asset processor|
|critical-section|`critical-section` provides the building blocks for synchronization primitives on all platforms, including `no_std`.|
|dds|DDS compressed texture support|
|debug_glam_assert|Enable assertions in debug builds to check the validity of parameters passed to glam|
|default_no_std|Recommended defaults for no_std applications|
|detailed_trace|Enable detailed trace event logging. These trace events are expensive even when off, thus they require compile time opt-in|
|dlss|NVIDIA Deep Learning Super Sampling|
|dynamic_linking|Force dynamic linking, which improves iterative compile times|
|embedded_watcher|Enables watching in memory asset providers for Bevy Asset hot-reloading|
|experimental_bevy_feathers|Feathers widget collection.|
|experimental_pbr_pcss|Enable support for PCSS, at the risk of blowing past the global, per-shader sampler limit on older/lower-end GPUs|
|exr|EXR image format support|
|ff|Farbfeld image format support|
|file_watcher|Enables watching the filesystem for Bevy Asset hot-reloading|
|flac|FLAC audio format support|
|force_disable_dlss|Forcibly disable DLSS so that cargo build --all-features works without the DLSS SDK being installed. Not meant for users.|
|ghost_nodes|Experimental support for nodes that are ignored for UI layouting|
|gif|GIF image format support|
|glam_assert|Enable assertions to check the validity of parameters passed to glam|
|hotpatching|Enable hotpatching of Bevy systems|
|http|Enables downloading assets from HTTP sources. Warning: there are security implications. Read the docs on WebAssetPlugin.|
|https|Enables downloading assets from HTTPS sources. Warning: there are security implications. Read the docs on WebAssetPlugin.|
|ico|ICO image format support|
|jpeg|JPEG image format support|
|libm|Uses the `libm` maths library instead of the one provided in `std` and `core`.|
|meshlet|Enables the meshlet renderer for dense high-poly scenes (experimental)|
|meshlet_processor|Enables processing meshes into meshlet meshes for bevy_pbr|
|mp3|MP3 audio format support|
|pbr_anisotropy_texture|Enable support for anisotropy texture in the `StandardMaterial`, at the risk of blowing past the global, per-shader texture limit on older/lower-end GPUs|
|pbr_clustered_decals|Enable support for Clustered Decals|
|pbr_light_textures|Enable support for Light Textures|
|pbr_multi_layer_material_textures|Enable support for multi-layer material textures in the `StandardMaterial`, at the risk of blowing past the global, per-shader texture limit on older/lower-end GPUs|
|pbr_specular_textures|Enable support for specular textures in the `StandardMaterial`, at the risk of blowing past the global, per-shader texture limit on older/lower-end GPUs|
|pbr_transmission_textures|Enable support for transmission-related textures in the `StandardMaterial`, at the risk of blowing past the global, per-shader texture limit on older/lower-end GPUs|
|pnm|PNM image format support, includes pam, pbm, pgm and ppm|
|qoi|QOI image format support|
|raw_vulkan_init|Forces the wgpu instance to be initialized using the raw Vulkan HAL, enabling additional configuration|
|reflect_auto_register_static|Enable automatic reflect registration without inventory. See `reflect::load_type_registrations` for more info.|
|reflect_documentation|Enable documentation reflection|
|reflect_functions|Enable function reflection|
|serialize|Enable serialization support through serde|
|shader_format_glsl|Enable support for shaders in GLSL|
|shader_format_spirv|Enable support for shaders in SPIR-V|
|shader_format_wesl|Enable support for shaders in WESL|
|spirv_shader_passthrough|Enable passthrough loading for SPIR-V shaders (Only supported on Vulkan, shader capabilities and extensions must agree with the platform implementation)|
|statically-linked-dxc|Statically linked DXC shader compiler for DirectX 12|
|symphonia-aac|AAC audio format support (through symphonia)|
|symphonia-all|AAC, FLAC, MP3, MP4, OGG/VORBIS, and WAV audio formats support (through symphonia)|
|symphonia-flac|FLAC audio format support (through symphonia)|
|symphonia-isomp4|MP4 audio format support (through symphonia)|
|symphonia-vorbis|OGG/VORBIS audio format support (through symphonia)|
|symphonia-wav|WAV audio format support (through symphonia)|
|tga|TGA image format support|
|tiff|TIFF image format support|
|trace|Tracing support|
|trace_chrome|Tracing support, saving a file in Chrome Tracing format|
|trace_tracy|Tracing support, exposing a port for Tracy|
|trace_tracy_memory|Tracing support, with memory profiling, exposing a port for Tracy|
|track_location|Enables source location tracking for change detection and spawning/despawning, which can assist with debugging|
|wav|WAV audio format support|
|web|Enables use of browser APIs. Note this is currently only applicable on `wasm32` architectures.|
|web_asset_cache|Enable caching downloaded assets on the filesystem. NOTE: this cache currently never invalidates entries!|
|webgpu|Enable support for WebGPU in Wasm. When enabled, this feature will override the `webgl2` feature and you won't be able to run Wasm builds with WebGL2, only with WebGPU.|
|webp|WebP image format support|
|zlib|For KTX2 supercompression|
|zstd_c|For KTX2 Zstandard decompression using [zstd](https://crates.io/crates/zstd). This is a faster backend, but uses unsafe C bindings. For the safe option, stick to the default backend with "zstd_rust".|
