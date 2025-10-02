title: "Remove FontAtlasSets"
pull_requests: [#21345]
---

`FontAtlasSets` has been removed. `FontAtlasKey` now wraps a `(AssetId<Font>, u32, FontSmoothing)`.
Font atlases are looked up directly using a `FontAtlasKey`, there's no seperate `AssetId<Font>` map.
