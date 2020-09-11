# Changelog

## Unreleased

### Added

- [Task System for Bevy][384]
  - Replaces rayon with a custom designed task system that consists of several "TaskPools".
  - Exports `IOTaskPool`, `ComputePool`, and `AsyncComputePool` in `bevy_tasks` crate.
- [Added support for binary glTF (.glb).][271]
- [Added support for 'or' in ECS querying for tuple queries.][358]
- [Added `Color::hex`][362] to create a `Color` from hex values.
  - supports RGB , RGBA, RRGGBB, and RRGGBBAA.
- [Added `Color::rgb_u8` and `Color::rgba_u8`.][381]
- [Added `bevy_render::pass::ClearColor` to prelude.][396]
- [Added methods on `Input<T>`][428] for access to all pressed/just_pressed/just_released keys.
  - `{get_pressed, get_just_pressed, get_just_released}`
- [Derived `Clone` for UI component bundles.][390]
- Tips for faster builds on macos: [#312][312], [#314][314], [#433][433]
- Added and documented cargo features
  - [Created document `docs/cargo_features.md`.][249]
  - [Added features for x11 and wayland display servers.][249]
  - [and added a feature to disable libloading.][363] (helpful for WASM support)
- Added more instructions for linux dependencies: [Arch / Manjaro][275], [NixOS][290], and [Solus][331]

### Changed
 
- [Bump entities to u128 to avoid collisions][393]
- [Send an AssetEvent when modifying using `get_id_mut`][323]
- [Rename `Assets::get_id_mut` -> `Assets::get_with_id_mut`][332]
- [Support multiline text in `DrawableText`][183]
- [Some examples of documentation][338]
- [iOS: use shaderc-rs for glsl to spirv compilation][324]
- [Changed the default node size to Auto instead of Undefined to match the Stretch implementation.][304]
- Many improvements to Bevy's CI [#325][325], [#349][349], [#357][357], [#373][373], [#423][423]
- [Load assets from root path when loading directly][478]

### Fixed

- [Properly track added and removed RenderResources in RenderResourcesNode.][361]
  - Fixes issues where entities vanished or changed color when new entities were spawned/despawned.
- [Fixed sprite clipping at same depth][385]; transparent sprites should no longer clip.
- [Check asset path existence][345]
- [Fixed deadlock in hot asset reloading][376]
- [Fixed hot asset reloading on Windows][394]
- [Allow glTFs to be loaded that don't have uvs and normals][406]
- [Fixed archetypes_generation being incorrectly updated for systems][383]
- [Remove child from parent when it is despawned][386]


[183]: https://github.com/bevyengine/bevy/pull/183
[249]: https://github.com/bevyengine/bevy/pull/249
[271]: https://github.com/bevyengine/bevy/pull/271
[275]: https://github.com/bevyengine/bevy/pull/275
[290]: https://github.com/bevyengine/bevy/pull/290
[304]: https://github.com/bevyengine/bevy/pull/304
[312]: https://github.com/bevyengine/bevy/pull/312
[314]: https://github.com/bevyengine/bevy/pull/314
[323]: https://github.com/bevyengine/bevy/pull/323
[324]: https://github.com/bevyengine/bevy/pull/324
[325]: https://github.com/bevyengine/bevy/pull/325
[331]: https://github.com/bevyengine/bevy/pull/331
[332]: https://github.com/bevyengine/bevy/pull/332
[338]: https://github.com/bevyengine/bevy/pull/332
[345]: https://github.com/bevyengine/bevy/pull/345
[349]: https://github.com/bevyengine/bevy/pull/349
[357]: https://github.com/bevyengine/bevy/pull/357
[358]: https://github.com/bevyengine/bevy/pull/358
[361]: https://github.com/bevyengine/bevy/pull/361
[362]: https://github.com/bevyengine/bevy/pull/362
[363]: https://github.com/bevyengine/bevy/pull/363
[373]: https://github.com/bevyengine/bevy/pull/373
[376]: https://github.com/bevyengine/bevy/pull/376
[381]: https://github.com/bevyengine/bevy/pull/381
[383]: https://github.com/bevyengine/bevy/pull/383
[384]: https://github.com/bevyengine/bevy/pull/384
[385]: https://github.com/bevyengine/bevy/pull/385
[386]: https://github.com/bevyengine/bevy/pull/386
[390]: https://github.com/bevyengine/bevy/pull/390
[393]: https://github.com/bevyengine/bevy/pull/393
[394]: https://github.com/bevyengine/bevy/pull/394
[396]: https://github.com/bevyengine/bevy/pull/396
[406]: https://github.com/bevyengine/bevy/pull/406
[423]: https://github.com/bevyengine/bevy/pull/423
[428]: https://github.com/bevyengine/bevy/pull/428
[433]: https://github.com/bevyengine/bevy/pull/433
[478]: https://github.com/bevyengine/bevy/pull/478


## Version 0.1.3 (2020-8-22)

## Version 0.1.2 (2020-8-10)

## Version 0.1.1 (2020-8-10)

## Version 0.1.0 (2020-8-10)
