---
title: Assets-as-entities
authors: ["@andriyDev"]
pull_requests: [22939]
---

In previous versions of Bevy, assets were stored in a big `Assets` source (per asset type).
Continuing our traditions, assets are now represented as ent\*ities! This allows us to remove some
bespoke implementations (like `AssetEvent`) and replace it with more generic ECS features (like
change detection or hooks/observers). We can now also take advantage of ECS features like
relationships in the implementation of assets.

While the simplification of Bevy internals is nice, what's more interesting is how users can
**also** benefit from these ECS features. For example, users can attach arbitrary components to
entities, whether that be to extend an asset's data, or to "tag" assets for better observability in
their apps. While users can take advantage of these today, their usage is still limited (for
example, users cannot add arbitrary components from within loaders, only from the ECS). We expect
this API to evolve to bring even more ergonomic access and more features to users!
