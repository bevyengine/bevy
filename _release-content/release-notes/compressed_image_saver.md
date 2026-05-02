---
title: CompressedImageSaver Improvements
authors: ["@JMS55", "@cwfitzgerald"]
pull_requests: [23567]
---

Bevy's `CompressedImageSaver` asset processor has been significantly upgraded with a new compression backend powered by the [`ctt`](https://github.com/cwfitzgerald/ctt) library.

The new `compressed_image_saver` feature compresses textures into BCn formats (for desktop GPUs) or ASTC formats (for mobile GPUs), producing higher-quality output than the previous Basis Universal approach. The compressor automatically selects the best output format based on the input texture's channel count and type — for example, single-channel textures get BC4, HDR textures get BC6H, and standard RGBA textures get BC7.

Try out the new `compressed_image_saver` example to see it in action.

## Automatic Mipmap Generation

No more manually generating mipmaps! The new backend automatically produces a full mip chain during compression. This means less aliasing when textures are viewed at a distance and better GPU cache utilization — all for free, just by running your textures through the asset processor.

## ASTC for Mobile

To target mobile GPUs, set the `BEVY_COMPRESSED_IMAGE_SAVER_ASTC` environment variable with your desired block size (e.g. `4x4`, `6x6`, `8x8`). Larger blocks give smaller files at the cost of quality. All 14 ASTC block sizes are supported.

## Basis Universal is Still Available

The previous Basis Universal compression behavior has been moved to the `compressed_image_saver_universal` feature. This remains the best choice for cross-platform distribution (including WebGPU), since UASTC can be transcoded at load time to whatever format the target GPU supports.
