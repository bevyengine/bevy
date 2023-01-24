# Cargo Features

## Default Features

|feature name|description|
|-|-|
|animation|Animation support and glTF animation loading.|
|bevy_asset|Provides asset functionality for Bevy Engine.|
|bevy_audio|Audio support. Support for all audio formats depends on this.|
|bevy_gilrs|Adds gamepad support.|
|bevy_gltf|[glTF](https://www.khronos.org/gltf/) support.|
|bevy_scene|Provides scene functionality for Bevy Engine.|
|bevy_winit|GUI support.|
|render|The render pipeline and all render related plugins.|
|png|PNG picture format support.|
|hdr|[HDR](https://en.wikipedia.org/wiki/High_dynamic_range) support.|
|vorbis|Ogg Vorbis audio format support.|
|x11|Make GUI applications use X11 protocol. You could enable wayland feature to override this.|
|filesystem_watcher|Enable watching the file system for asset hot reload|

## Optional Features

|feature name|description|
|-|-|
|bevy_dynamic_plugin|Plugin for dynamic loading (using [libloading](https://crates.io/crates/libloading)).|
|dynamic_linking|Forces bevy to be dynamically linked, which improves iterative compile times.|
|trace|Enables system tracing.|
|trace_chrome|Enables [tracing-chrome](https://github.com/thoren-d/tracing-chrome) as bevy_log output. This allows you to visualize system execution.|
|trace_tracy|Enables [Tracy](https://github.com/wolfpld/tracy) as bevy_log output. This allows `Tracy` to connect to and capture profiling data as well as visualize system execution in real-time, present statistics about system execution times, and more.|
|wgpu_trace|For tracing wgpu.|
|dds|DDS picture format support.|
|ktx2|KTX2 picture format support.|
|zlib|KTX2 Zlib supercompression support.|
|zstd|KTX2 Zstandard supercompression support.|
|basis-universal|Basis Universal picture format support and, if the `ktx2` feature is enabled, also KTX2 UASTC picture format transcoding support.|
|tga|TGA picture format support.|
|jpeg|JPEG picture format support.|
|bmp|BMP picture format support.|
|flac|FLAC audio format support. It's included in bevy_audio feature.|
|mp3|MP3 audio format support.|
|wav|WAV audio format support.|
|symphonia-aac|AAC audio format support by Symphonia. For more details, see `symphonia-all`.|
|symphonia-all|AAC, FLAC, MP4, MP3, Vorbis, and WAV support by Symphonia. Add support for parsing multiple file formats using a single crate instead of compiling different crates. The other non-`symphonia` features are disabled when its corresponding `symphonia` feature is enabled. [Link to `symphonia` documentation](https://docs.rs/symphonia/latest/symphonia/). More information about this topic can be found [here](https://github.com/bevyengine/bevy/pull/6388#discussion_r1009622883) |
|symphonia-flac|FLAC audio format support by Symphonia. For more details, see `symphonia-all`.|
|symphonia-isomp4|MP4 audio format support by Symphonia. For more details, see `symphonia-all`.|
|symphonia-mp3|MP3 audio format support by Symphonia. For more details, see `symphonia-all`.|
|symphonia-vorbis|Vorbis audio format support by Symphonia. For more details, see `symphonia-all`.|
|symphonia-wav|WAV audio format support by Symphonia. For more details, see `symphonia-all`.|
|serialize|Enables serialization of `bevy_input` types.|
|wayland|Enable this to use Wayland display server protocol other than X11.|
|subpixel_glyph_atlas|Enable this to cache glyphs using subpixel accuracy. This increases texture memory usage as each position requires a separate sprite in the glyph atlas, but provide more accurate character spacing.|
|bevy_ci_testing|Used for running examples in CI.|
|debug_asset_server|Enabling this turns on "hot reloading" of built in assets, such as shaders.|
