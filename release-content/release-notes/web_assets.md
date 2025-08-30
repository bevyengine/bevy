---
title: Web Assets
authors: ["@johanhelsing", "@mrchantey", "@jf908", "@atlv24"]
pull_requests: [20628]
---

Bevy now supports downloading assets from the web over http and https.
Use the new `http` and `https` features to enable `http://` and `https://` URLs as asset paths.
This functionality is powered by the [`ureq`](https://github.com/algesten/ureq) crate on native platforms and the fetch API on wasm.

```rust
let image = asset_server.load("https://example.com/image.png");
commands.spawn(Sprite::from_image(image));
```

Security note: if using web assets, be careful about where your URLs are coming from! If you allow arbitrary URLs to enter the asset server, it can potentially be exploited by an attacker to trigger vulnerabilities in our asset loaders, or DOS by downloading enormous files. We are not aware of any such vulnerabilities at the moment, just be careful!

By default these assets aren’t saved anywhere but you can enable the `web_asset_cache` feature to cache assets on your file system.

The implementation has changed quite a bit but this feature originally started out as an upstreaming of the [`bevy_web_asset`](https://github.com/johanhelsing/bevy_web_asset) crate.
Special thanks to @johanhelsing and bevy_web_asset's contributors!
