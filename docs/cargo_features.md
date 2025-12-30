<!-- MD041 - This file will be included in docs and should not start with a top header -->
<!-- Use 'cargo run -p build-templated-pages -- update features' to generate this file -->
<!-- markdownlint-disable-file MD041 -->

## Cargo Features

Bevy exposes many Cargo features to customize the engine. Enabling them adds functionality but may come at the cost of longer compilation times
and extra dependencies.

### Profiles

"Profiles" are high-level groups of cargo features that provide the full Bevy experience, but scoped to a specific domain.
These exist to be paired with `default-features = false`, enabling compiling only the subset of Bevy that you need.
This can cut down compile times and shrink your final binary size.

For example, you can compile only the "2D" Bevy features (without the 3D features) like this:

```toml
bevy = { version = "0.17", default-features = false, features = ["2d"] }
```

|Profile|Description|
|-|-|
|default|The full default Bevy experience. This is a combination of the following profiles: 2d, 3d, ui|
|2d|The default 2D Bevy experience. This includes the core Bevy framework, 2D functionality, Bevy UI, scenes, audio, and picking.|
|3d|The default 3D Bevy experience. This includes the core Bevy framework, 3D functionality, Bevy UI, scenes, audio, and picking.|
|ui|The default Bevy UI experience.  This includes the core Bevy framework, Bevy UI, scenes, audio, and picking.|

By default, the `bevy` crate enables the  features.

### Collections

"Collections" are mid-level groups of cargo features. These are used to compose the high-level "profiles". If the default profiles don't
suit your use case (ex: you want to use a custom renderer, you want to build a "headless" app, you want to target no_std, etc), then you can use these
collections to build your own "profile" equivalent, without needing to manually manage _every single_ feature.

|Collection|Description|
|-|-|
|dev|Enable this feature during development to improve the development experience. This adds features like asset hot-reloading and debugging tools. This should not be enabled for published apps!|
|audio|Features used to build audio Bevy apps.|
|scene|Features used to compose Bevy scenes.|
|picking|Enables picking functionality|
|default_app|The core pieces that most apps need. This serves as a baseline feature set for other higher level feature collections (such as "2d" and "3d"). It is also useful as a baseline feature set for scenarios like headless apps that require no rendering (ex: command line tools, servers, etc).|
|default_platform|These are platform support features, such as OS support/features, windowing and input backends, etc.|
|common_api|Default scene definition features. Note that this does not include an actual renderer, such as bevy_render (Bevy's default render backend).|
|2d_api|Features used to build 2D Bevy apps (does not include a render backend). You generally don't need to worry about this unless you are using a custom renderer.|
|2d_bevy_render|Bevy's built-in 2D renderer, built on top of `bevy_render`.|
|3d_api|Features used to build 3D Bevy apps (does not include a render backend). You generally don't need to worry about this unless you are using a custom renderer.|
|3d_bevy_render|Bevy's built-in 3D renderer, built on top of `bevy_render`.|
|ui_api|Features used to build UI Bevy apps (does not include a render backend). You generally don't need to worry about this unless you are using a custom renderer.|
|ui_bevy_render|Bevy's built-in UI renderer, built on top of `bevy_render`.|
|default_no_std|Recommended defaults for no_std applications|

### Feature List

This is the complete `bevy` cargo feature list, without "profiles" or "collections" (sorted by name):

|Feature|Description|
|-|-|
|accesskit_unix|Enable AccessKit on Unix backends (currently only works with experimental screen readers and forks.)|
|android-game-activity|Android GameActivity support. Default, choose between this and `android-native-activity`.|
|android-native-activity|Android NativeActivity support. Legacy, should be avoided for most new Android games.|
|android_shared_stdcxx|Enable using a shared stdlib for cxx on Android|
|asset_processor|Enables the built-in asset processor for processed assets.|
|async-io|Use async-io's implementation of block_on instead of futures-lite's implementation. This is preferred if your application uses async-io.|
|async_executor|Uses `async-executor` as a task execution backend.|
|basis-universal|Basis Universal compressed texture support|
|bevy_animation|Provides animation functionality|
|bevy_anti_alias|Provides various anti aliasing solutions|
|bevy_asset|Provides asset functionality|
|bevy_audio|Provides audio functionality|
|bevy_camera|Provides camera and visibility types, as well as culling primitives.|
|bevy_camera_controller|Provides a collection of prebuilt camera controllers|
|bevy_ci_testing|Enable systems that allow for automated testing on CI|
|bevy_color|Provides shared color types and operations|
|bevy_core_pipeline|Provides cameras and other basic render pipeline features|
|bevy_debug_stepping|Enable stepping-based debugging of Bevy systems|
|bevy_dev_tools|Provides a collection of developer tools|
|bevy_gilrs|Adds gamepad support|
|bevy_gizmos|Adds support for gizmos|
|bevy_gizmos_render|Adds support for rendering gizmos|
|bevy_gltf|[glTF](https://www.khronos.org/gltf/) support|
|bevy_image|Load and access image data. Usually added by an image format|
|bevy_input_focus|Enable input focus subsystem|
|bevy_light|Provides light types such as point lights, directional lights, spotlights.|
|bevy_log|Enable integration with `tracing` and `log`|
|bevy_mesh|Provides a mesh format and some primitive meshing routines.|
|bevy_mikktspace|Provides vertex tangent generation for use with bevy_mesh.|
|bevy_pbr|Adds PBR rendering|
|bevy_picking|Provides picking functionality|
|bevy_post_process|Provides post process effects such as depth of field, bloom, chromatic aberration.|
|bevy_remote|Enable the Bevy Remote Protocol|
|bevy_render|Provides rendering functionality|
|bevy_scene|Provides scene functionality|
|bevy_shader|Provides shaders usable through asset handles.|
|bevy_solari|Provides raytraced lighting (experimental)|
|bevy_sprite|Provides sprite functionality|
|bevy_sprite_render|Provides sprite rendering functionality|
|bevy_state|Enable built in global state machines|
|bevy_text|Provides text functionality|
|bevy_ui|A custom ECS-driven UI framework|
|bevy_ui_debug|Provides a debug overlay for bevy UI|
|bevy_ui_render|Provides rendering functionality for bevy_ui|
|bevy_window|Windowing layer|
|bevy_winit|winit window and input backend|
|bluenoise_texture|Include spatio-temporal blue noise KTX2 file used by generated environment maps, Solari and atmosphere|
|bmp|BMP image format support|
|compressed_image_saver|Enables compressed KTX2 UASTC texture output on the asset processor|
|critical-section|`critical-section` provides the building blocks for synchronization primitives on all platforms, including `no_std`.|
|custom_cursor|Enable winit custom cursor support|
|dds|DDS compressed texture support|
|debug|Enable collecting debug information about systems and components to help with diagnostics|
|debug_glam_assert|Enable assertions in debug builds to check the validity of parameters passed to glam|
|default_font|Include a default font, containing only ASCII characters, at the cost of a 20kB binary size increase|
|detailed_trace|Enable detailed trace event logging. These trace events are expensive even when off, thus they require compile time opt-in|
|dlss|NVIDIA Deep Learning Super Sampling|
|dynamic_linking|Force dynamic linking, which improves iterative compile times|
|embedded_watcher|Enables watching in memory asset providers for Bevy Asset hot-reloading|
|experimental_bevy_feathers|Feathers widget collection.|
|experimental_bevy_ui_widgets|Experimental headless widget collection for Bevy UI.|
|experimental_pbr_pcss|Enable support for PCSS, at the risk of blowing past the global, per-shader sampler limit on older/lower-end GPUs|
|exr|EXR image format support|
|ff|Farbfeld image format support|
|file_watcher|Enables watching the filesystem for Bevy Asset hot-reloading|
|flac|FLAC audio format support|
|force_disable_dlss|Forcibly disable DLSS so that cargo build --all-features works without the DLSS SDK being installed. Not meant for users.|
|free_camera|Enables the free cam from bevy_camera_controller|
|gamepad|Gamepad support. Automatically enabled by `bevy_gilrs`.|
|gestures|Gestures support. Automatically enabled by `bevy_window`.|
|ghost_nodes|Experimental support for nodes that are ignored for UI layouting|
|gif|GIF image format support|
|glam_assert|Enable assertions to check the validity of parameters passed to glam|
|gltf_animation|Enable glTF animation loading|
|hdr|HDR image format support|
|hotpatching|Enable hotpatching of Bevy systems|
|http|Enables downloading assets from HTTP sources. Warning: there are security implications. Read the docs on WebAssetPlugin.|
|https|Enables downloading assets from HTTPS sources. Warning: there are security implications. Read the docs on WebAssetPlugin.|
|ico|ICO image format support|
|jpeg|JPEG image format support|
|keyboard|Keyboard support. Automatically enabled by `bevy_window`.|
|ktx2|KTX2 compressed texture support|
|libm|Uses the `libm` maths library instead of the one provided in `std` and `core`.|
|mesh_picking|Provides an implementation for picking meshes|
|meshlet|Enables the meshlet renderer for dense high-poly scenes (experimental)|
|meshlet_processor|Enables processing meshes into meshlet meshes for bevy_pbr|
|morph|Enables support for morph target weights in bevy_mesh|
|morph_animation|Enables bevy_mesh and bevy_animation morph weight support|
|mouse|Mouse support. Automatically enabled by `bevy_window`.|
|mp3|MP3 audio format support|
|multi_threaded|Enables multithreaded parallelism in the engine. Disabling it forces all engine tasks to run on a single thread.|
|pan_camera|Enables the pan camera from bevy_camera_controller|
|pbr_anisotropy_texture|Enable support for anisotropy texture in the `StandardMaterial`, at the risk of blowing past the global, per-shader texture limit on older/lower-end GPUs|
|pbr_clustered_decals|Enable support for Clustered Decals|
|pbr_light_textures|Enable support for Light Textures|
|pbr_multi_layer_material_textures|Enable support for multi-layer material textures in the `StandardMaterial`, at the risk of blowing past the global, per-shader texture limit on older/lower-end GPUs|
|pbr_specular_textures|Enable support for specular textures in the `StandardMaterial`, at the risk of blowing past the global, per-shader texture limit on older/lower-end GPUs|
|pbr_transmission_textures|Enable support for transmission-related textures in the `StandardMaterial`, at the risk of blowing past the global, per-shader texture limit on older/lower-end GPUs|
|png|PNG image format support|
|pnm|PNM image format support, includes pam, pbm, pgm and ppm|
|qoi|QOI image format support|
|raw_vulkan_init|Forces the wgpu instance to be initialized using the raw Vulkan HAL, enabling additional configuration|
|reflect_auto_register|Enable automatic reflect registration|
|reflect_auto_register_static|Enable automatic reflect registration without inventory. See `reflect::load_type_registrations` for more info.|
|reflect_documentation|Enables bevy_reflect to access documentation comments of rust code at runtime|
|reflect_functions|Enable function reflection|
|serialize|Enable serialization support through serde|
|shader_format_glsl|Enable support for shaders in GLSL|
|shader_format_spirv|Enable support for shaders in SPIR-V|
|shader_format_wesl|Enable support for shaders in WESL|
|smaa_luts|Include SMAA Look Up Tables KTX2 Files|
|spirv_shader_passthrough|Enable passthrough loading for SPIR-V shaders (Only supported on Vulkan, shader capabilities and extensions must agree with the platform implementation)|
|sprite_picking|Provides an implementation for picking sprites|
|statically-linked-dxc|Statically linked DXC shader compiler for DirectX 12|
|std|Allows access to the `std` crate.|
|symphonia-aac|AAC audio format support (through symphonia)|
|symphonia-all|AAC, FLAC, MP3, MP4, OGG/VORBIS, and WAV audio formats support (through symphonia)|
|symphonia-flac|FLAC audio format support (through symphonia)|
|symphonia-isomp4|MP4 audio format support (through symphonia)|
|symphonia-vorbis|OGG/VORBIS audio format support (through symphonia)|
|symphonia-wav|WAV audio format support (through symphonia)|
|sysinfo_plugin|Enables system information diagnostic plugin|
|tga|TGA image format support|
|tiff|TIFF image format support|
|tonemapping_luts|Include tonemapping Look Up Tables KTX2 files. If everything is pink, you need to enable this feature or change the `Tonemapping` method for your `Camera2d` or `Camera3d`.|
|touch|Touch support. Automatically enabled by `bevy_window`.|
|trace|Tracing support|
|trace_chrome|Tracing support, saving a file in Chrome Tracing format|
|trace_tracy|Tracing support, exposing a port for Tracy|
|trace_tracy_memory|Tracing support, with memory profiling, exposing a port for Tracy|
|track_location|Enables source location tracking for change detection and spawning/despawning, which can assist with debugging|
|ui_picking|Provides an implementation for picking UI|
|vorbis|OGG/VORBIS audio format support|
|wav|WAV audio format support|
|wayland|Wayland display server support|
|web|Enables use of browser APIs. Note this is currently only applicable on `wasm32` architectures.|
|web_asset_cache|Enable caching downloaded assets on the filesystem. NOTE: this cache currently never invalidates entries!|
|webgl2|Enable some limitations to be able to use WebGL2. Please refer to the [WebGL2 and WebGPU](https://github.com/bevyengine/bevy/tree/latest/examples#webgl2-and-webgpu) section of the examples README for more information on how to run Wasm builds with WebGPU.|
|webgpu|Enable support for WebGPU in Wasm. When enabled, this feature will override the `webgl2` feature and you won't be able to run Wasm builds with WebGL2, only with WebGPU.|
|webp|WebP image format support|
|x11|X11 display server support|
|zlib|For KTX2 supercompression|
|zstd_c|For KTX2 Zstandard decompression using [zstd](https://crates.io/crates/zstd). This is a faster backend, but uses unsafe C bindings. For the safe option, stick to the default backend with "zstd_rust".|
|zstd_rust|For KTX2 Zstandard decompression using pure rust [ruzstd](https://crates.io/crates/ruzstd). This is the safe default. For maximum performance, use "zstd_c".|
