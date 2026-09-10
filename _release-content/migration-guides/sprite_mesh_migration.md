---
title: Migrate Sprite to use Mesh2d + SpriteMaterial
pull_requests: [25432]
---

The `Sprite` rendering backend was migrated to use the `Mesh2d` and `Material2d` infrastructure.

The `Sprite` component now has a new `alpha_mode` field but is otherwise unchanged.
It defaults to `Blend` which is what the old backend was using but now you can use `Opaque` or `Mask(f32)` when it makes sense for your use case.

The old backend has not been removed yet since `Text2d` still relies on it but if you were relying on it you should consider moving away from it.

The draw order for `Sprite` that shared the same Z-level might be different in some cases.
In general, you should always specify a Z-level to any sprite that could overlap but it's possible it worked before but that was not part of the contract.
