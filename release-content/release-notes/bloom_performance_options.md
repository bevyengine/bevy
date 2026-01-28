---
title: Add performance options to Bloom.
authors: ["@beicause"]
pull_requests: [21340]
---

Bloom is a relatively expensive post-processing for low-end devices, as it requires multiple render passes for downsampling and upsampling. For more performance configurability, we added the `high_quality` (default: true) and `max_mip_count` (default: unlimited) options to Bloom, in addition to the existing `max_mip_dimension`.

If `high_quality` is false, Bloom will use a faster but lower quality implementation, which significantly reduces texture sampling but still maintains reasonable visual quality. For low-end devices, this could potentially reduce frame time by a few milliseconds.

You can also set `max_mip_count` and/or `max_mip_dimension` to a lower value for a significant performance gain. By default the bloom texture has a maximum short-side size of 512 and uses all 8 mipmaps. You may be able to cut the Bloom frame time in half by reducing the mipmap count to a smaller value (such as 3 or 4). However, please note that these two options impact the bloom quality and need to be balanced for your needs.
