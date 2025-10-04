---
title: Removed `FontAtlasSets`
pull_requests: [21345]
---

* `FontAtlasSets` has been removed. 
* `FontAtlasKey` now wraps a `(AssetId<Font>, u32, FontSmoothing)`.
* Font atlases are looked up directly using a `FontAtlasKey`, there's no separate `AssetId<Font>` map.
* `remove_dropped_font_atlas_sets` has been renamed `free_unused_font_atlases_system`.
* `FontAtlasSet` is now a newtype over a hashmap implementing `Deref` and `DerefMut`.
