---
title: "`bevy_text` migration from Cosmic Text to Parley"
pull_requests: [22879]
---

`bevy_text` now uses Parley for its text layout. For the most part, this change should be invisible to users of `bevy_text` and Bevy more broadly.

However, some low-level public methods and types (such as `FontAtlasKey`) have changed to map to `parley`'s distinct API.

This migration should be relatively straightforward. Use the linked PR as an example of the correct migration, but please ask for help (and explain your use case) if you run into difficulties not noted below.

Known migration steps:

- System font discovery now requires you to enable the `bevy/system_font_discovery` feature. Users on Linux will need the `fontconfig` library for this. On Ubuntu, this can be done using `sudo apt install libfontconfig1-dev`.
- The various methods for setting the fallback font (such as `set_serif_family`, `set_sans_serif_family` or `set_monospace_family`) now return a `Result`. These will fail if the provided font is not found. By-and-large, you should not need to call these methods: font fallback is handled automatically via `fontique` through `parley`, using the system-provided fallback fonts (but see the above note about system font discovery).
