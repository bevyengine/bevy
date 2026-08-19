---
title: "Color-primaries metadata for image assets"
authors: ["@stuartparmenter"]
pull_requests: [25472]
---

Image assets now carry their source gamut in the new `Image::source_primaries` field.
The supported primary sets are `Bt709`, the default, `Bt2020`, and `DisplayP3`.
The KTX2, PNG, Radiance HDR, and OpenEXR loaders read it from file metadata, and a new
`source_primaries` loader setting overrides it per asset, in code or in a `.meta` file.
For now this is metadata only, so decoding and rendering are unchanged.
