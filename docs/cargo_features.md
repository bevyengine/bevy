# Cargo Features

## Default Features

### bevy_audio

Audio support. All audio formats support depends on this.

### bevy_gltf

[glTF](https://www.khronos.org/gltf/) support.

### bevy_winit

GUI support.

### bevy_wgpu

Make use of GPU via [WebGPU](https://gpuweb.github.io/gpuweb/) support.

### render
The render pipeline and all render related plugins.

### dynamic_plugins
Plugins for dynamic loading (libloading)

### png 

PNG picture format support. 

### hdr

[HDR](https://en.wikipedia.org/wiki/High_dynamic_range) support.

### mp3

Audio of mp3 format support.

### x11

Make GUI applications use X11 procotol. You could enable wayland feature to override this.

## Optional Features

### profiler

For profiler.

### wgpu_trace

For tracing wgpu.

### flac

FLAC audio fromat support. It's included in bevy_audio feature.

### wav

WAV audio format support.

### vorbis 

Vorbis audio format support.

### wayland

Enable this to use Wayland display server protocol other than X11.