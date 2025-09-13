---
title: Stop exposing mp3 support through minimp3
pull_requests: [20183]
---

The `minimp3` feature is no longer exposed from Bevy. Bevy still supports mp3 through the `mp3` feature.

If you were relying on something specific to `minimp3`, you can still enable it by adding a dependency to `rodio` with the `minimp3` feature:

```toml
[dependencies]
rodio = { version = "0.20", features = ["minimp3"] }
```

This is best to avoid though, as `minimp3` is not actively maintained, doesn't work in wasm, has been known to cause application rejection from the Apple App Store, and has a few security vulnerabilities.
