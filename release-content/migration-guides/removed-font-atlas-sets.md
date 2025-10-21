---
title: Removed `FontAtlasSets`
pull_requests: [21345]
---

* `FontAtlasSets` has been removed.
* `FontAtlasKey` now newtypes a `(AssetId<Font>, u32, FontSmoothing)`.
* `FontAtlasSet` is now a resource. It newtypes a `HashMap<FontAtlasKey, Vec<FontAtlas>>` and derives `Deref` and `DerefMut`.
* Font atlases are looked up directly using a `FontAtlasKey`, there's no longer a separate `AssetId<Font>` to `FontAtlasKey` map.
* `remove_dropped_font_atlas_sets` has been renamed to `free_unused_font_atlases_system`.
* The `FontAtlasSet` methods `add_glyph_to_atlas`, `get_glyph_atlas_info`, and `get_outlined_glyph_texture` have been moved into the `font_atlas` module and reworked into free functions.
