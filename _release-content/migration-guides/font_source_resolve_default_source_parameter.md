---
title: New `default_source` parameter on `FontSource::resolve_font_family`
pull_requests: [25847]
---

The `FontSource::resolve_font_family` method now takes a `default_source` argument that is used to resolve `FontSource::Default`.

You should set the value of the `DefaultFontSource` resource to set the default font for your app now.
