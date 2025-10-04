---
title: Removed `FontAtlasSets`
pull_requests: [21345]
---

* `FontAtlasSets` has been removed. 
* `FontAtlasKey` now newtypes a `(AssetId<Font>, u32, FontSmoothing)`.
* `FontAtlasSet` is now a resource. It newtypes a `HashMap<FontAtlasKey, Vec<FontAtlas>>` and derives `Deref` and `DerefMut`.
* Font atlases are looked up directly using a `FontAtlasKey`, there's no longer a separate `AssetId<Font>` to `FontAtlasKey` map.
* `remove_dropped_font_atlas_sets` has been renamed to `free_unused_font_atlases_system`.
