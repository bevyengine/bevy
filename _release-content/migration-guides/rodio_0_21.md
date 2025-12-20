---
title: "Rodio 0.21 Update"
pull_requests: [20323]
---

`rodio` was updated to `0.21` and `cpal` to `0.17`. The following sections will guide you through the necessary changes to ensure compatibility.

## Audio Feature Flags

Audio format related features were reworked with this update.

By default, Bevy will enable the `vorbis` feature, which supports OGG/VORBIS files through `lewton`.

If you are not using Bevy's default features, here's a list you can use for reference:

- `vorbis`: OGG/VORBIS audio format support (through `lewton`).
- `wav`: WAV audio format support (through `hound`).
- `mp3`: MP3 audio format support (through `symphonia`).
- `mp4`: MP4 audio format support (through `symphonia`). It also enables AAC support.
- `flac`: FLAC audio format support (through `claxon`).
- `aac`: AAC audio format support (through `symphonia`).

There are also specific `symphonia` backend flags you can use for certain formats instead of the default flags:

- `symphonia-flac`
- `symphonia-vorbis`
- `symphonia-wav`

Notice that OGG/VORBIS support through `symphonia` is currently subject to issues with buffering, reverb, looping and spatial audio. Check the following issues/PRs for additional context:

- <https://github.com/RustAudio/rodio/issues/775>
- <https://github.com/RustAudio/rodio/pull/786>

The `audio-all` feature was added for convenience. It will enable all the available audio formats through their default backends.

## Audio Traits

`type DecoderItem` was removed from the `Decodable` trait. Now `rodio::Sample` is an alias for `f32`.

## Android Related Features

The `android_shared_stdcxx` feature was removed, as `cpal`'s `oboe-shared-stdcxx` feature was also removed in favor of Android NDK audio APIs.

Keep in mind that if you are using `bevy_audio` the minimum supported Android API version is now 26 (Android 8/Oreo).
