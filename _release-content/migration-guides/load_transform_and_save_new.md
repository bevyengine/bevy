---
title: LoadTransformAndSave::new now takes an AssetLoader argument.
pull_requests: [26091]
---

`LoadTransformAndSave::new` now takes an `AssetLoader` value as an argument. You can keep the old
behavior by calling `LoadTransformAndSave::with_implicit_loader` instead.
