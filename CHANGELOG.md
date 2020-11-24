# Changelog

While we try to keep the `Unreleased` changes updated, it is often behind and does not include
all merged pull requests. To see a list of all changes since the latest release, you may compare
current changes on git with [previous release tags][git_tag_comparison].

[git_tag_comparison]: https://github.com/bevyengine/bevy/compare/v0.3.0...master


## Unreleased

### Added

### Changed

### Fixed


## Version 0.3.0 (2020-11-03)

### Added

- [Touch Input][696]
- [iOS XCode Project][539]
- [Android Example and use bevy-glsl-to-spirv 0.2.0][740]
- [Introduce Mouse capture API][679]
- [`bevy_input::touch`: implement touch input][696]
- [D-pad support on MacOS][653]
- [Support for Android file system][723]
- [app: PluginGroups and DefaultPlugins][744]
  - `PluginGroup` is a collection of plugins where each plugin can be enabled or disabled.
- [Support to get gamepad button/trigger values using `Axis<GamepadButton>`][683]
- [Expose Winit decorations][627]
- [Enable changing window settings at runtime][644]
- [Expose a pointer of EventLoopProxy to process custom messages][674]
- [Add a way to specify padding/ margins between sprites in a TextureAtlas][460]
- [Add `bevy_ecs::Commands::remove` for bundles][579]
- [impl `Default` for `TextureFormat`][675]
- [Expose current_entity in ChildBuilder][595]
- [`AppBuilder::add_thread_local_resource`][671]
- [`Commands::write_world_boxed` takes a pre-boxed world writer to the ECS's command queue][661]
- [`FrameTimeDiagnosticsPlugin` now shows "frame count" in addition to "frame time" and "fps"][678]
- [Add hierarchy example][565]
- [`WgpuPowerOptions` for choosing between low power, high performance, and adaptive power][397]
- Derive `Debug` for more types: [#597][597], [#632][632] 
- Index buffer specialization
  - [Allows the use of U32 indices in Mesh index buffers in addition to the usual U16 indices][568]
  - [Switch to u32 indices by default][572]
- More instructions for system dependencies
  - [Add `systemd-devel` for Fedora Linux dependencies][528]
  - [Add `libudev-dev` to Ubuntu dependencies][538]
  - [Add Void Linux to linux dependencies file][645]
  - [WSL2 instructions][727]
- [Suggest `-Zrun-dsymutil-no` for faster compilation on MacOS][552]

### Changed

- [ecs: ergonomic query.iter(), remove locks, add QuerySets][741]
  - `query.iter()` is now a real iterator!
  - `QuerySet` allows working with conflicting queries and is checked at compile-time.
- [Rename `query.entity()` and `query.get()`][752]
  - `query.get::<Component>(entity)` is now `query.get_component::<Component>(entity)`
  - `query.entity(entity)` is now `query.get(entity)`
- [Asset system rework and GLTF scene loading][693]
- [Introduces WASM implementation of `AssetIo`][703]
- [Move transform data out of Mat4][596]
- [Separate gamepad state code from gamepad event code and other customizations][700]
- [gamepad: expose raw and filtered gamepad events][711]
- [Do not depend on `spirv-reflect` on `wasm32` target][689]
- [Move dynamic plugin loading to its own optional crate][544]
- [Add field to `WindowDescriptor` on wasm32 targets to optionally provide an existing canvas element as winit window][515]
- [Adjust how `ArchetypeAccess` tracks mutable & immutable deps][660]
- [Use `FnOnce` in `Commands` and `ChildBuilder` where possible][535]
- [Runners explicitly call `App.initialize()`][690]
- [sRGB awareness for `Color`][616]
  - Color is now assumed to be provided in the non-linear sRGB colorspace.
    Constructors such as `Color::rgb` and `Color::rgba` will be converted to linear sRGB.
  - New methods `Color::rgb_linear` and `Color::rgba_linear` will accept colors already in linear sRGB (the old behavior)
  - Individual color-components must now be accessed through setters and getters.
- [`Mesh` overhaul with custom vertex attributes][599]
  - Any vertex attribute can now be added over `mesh.attributes.insert()`.
  - See `example/shader/mesh_custom_attribute.rs`
  - Removed `VertexAttribute`, `Vertex`, `AsVertexBufferDescriptor`.
  - For missing attributes (requested by shader, but not defined by mesh), Bevy will provide a zero-filled fallback buffer.
- Despawning an entity multiple times causes a debug-level log message to be emitted instead of a panic: [#649][649], [#651][651]
- [Migrated to Rodio 0.12][692]
  - New method of playing audio can be found in the examples.
- Added support for inserting custom initial values for `Local<T>` system resources [#745][745]
  
### Fixed

- [Properly update bind group ids when setting dynamic bindings][560]
- [Properly exit the app on AppExit event][610]
- [Fix FloatOrd hash being different for different NaN values][618]
- [Fix Added behavior for QueryOne get][543]
- [Update camera_system to fix issue with late camera addition][488]
- [Register `IndexFormat` as a property][664]
- [Fix breakout example bug][685]
- [Fix PreviousParent lag by merging parent update systems][713]
- [Fix bug of connection event of gamepad at startup][730]
- [Fix wavy text][725]

[397]: https://github.com/bevyengine/bevy/pull/397
[460]: https://github.com/bevyengine/bevy/pull/460
[488]: https://github.com/bevyengine/bevy/pull/488
[515]: https://github.com/bevyengine/bevy/pull/515
[528]: https://github.com/bevyengine/bevy/pull/528
[535]: https://github.com/bevyengine/bevy/pull/535
[538]: https://github.com/bevyengine/bevy/pull/538
[539]: https://github.com/bevyengine/bevy/pull/539
[543]: https://github.com/bevyengine/bevy/pull/543
[544]: https://github.com/bevyengine/bevy/pull/544
[552]: https://github.com/bevyengine/bevy/pull/552
[560]: https://github.com/bevyengine/bevy/pull/560
[565]: https://github.com/bevyengine/bevy/pull/565
[568]: https://github.com/bevyengine/bevy/pull/568
[572]: https://github.com/bevyengine/bevy/pull/572
[579]: https://github.com/bevyengine/bevy/pull/579
[595]: https://github.com/bevyengine/bevy/pull/595
[596]: https://github.com/bevyengine/bevy/pull/596
[597]: https://github.com/bevyengine/bevy/pull/597
[599]: https://github.com/bevyengine/bevy/pull/599
[610]: https://github.com/bevyengine/bevy/pull/610
[616]: https://github.com/bevyengine/bevy/pull/616
[618]: https://github.com/bevyengine/bevy/pull/618
[627]: https://github.com/bevyengine/bevy/pull/627
[632]: https://github.com/bevyengine/bevy/pull/632
[644]: https://github.com/bevyengine/bevy/pull/644
[645]: https://github.com/bevyengine/bevy/pull/645
[649]: https://github.com/bevyengine/bevy/pull/649
[651]: https://github.com/bevyengine/bevy/pull/651
[653]: https://github.com/bevyengine/bevy/pull/653
[660]: https://github.com/bevyengine/bevy/pull/660
[661]: https://github.com/bevyengine/bevy/pull/661
[664]: https://github.com/bevyengine/bevy/pull/664
[671]: https://github.com/bevyengine/bevy/pull/671
[674]: https://github.com/bevyengine/bevy/pull/674
[675]: https://github.com/bevyengine/bevy/pull/675
[678]: https://github.com/bevyengine/bevy/pull/678
[679]: https://github.com/bevyengine/bevy/pull/679
[683]: https://github.com/bevyengine/bevy/pull/683
[685]: https://github.com/bevyengine/bevy/pull/685
[689]: https://github.com/bevyengine/bevy/pull/689
[690]: https://github.com/bevyengine/bevy/pull/690
[692]: https://github.com/bevyengine/bevy/pull/692
[693]: https://github.com/bevyengine/bevy/pull/693
[696]: https://github.com/bevyengine/bevy/pull/696
[700]: https://github.com/bevyengine/bevy/pull/700
[703]: https://github.com/bevyengine/bevy/pull/703
[711]: https://github.com/bevyengine/bevy/pull/711
[713]: https://github.com/bevyengine/bevy/pull/713
[723]: https://github.com/bevyengine/bevy/pull/723
[725]: https://github.com/bevyengine/bevy/pull/725
[727]: https://github.com/bevyengine/bevy/pull/727
[730]: https://github.com/bevyengine/bevy/pull/730
[740]: https://github.com/bevyengine/bevy/pull/740
[741]: https://github.com/bevyengine/bevy/pull/741
[744]: https://github.com/bevyengine/bevy/pull/744
[745]: https://github.com/bevyengine/bevy/pull/745
[752]: https://github.com/bevyengine/bevy/pull/752


## Version 0.2.1 (2020-9-20)

### Fixed

- [Remove UI queue print][521]
- [Use async executor 1.3.0][526]

[521]: https://github.com/bevyengine/bevy/pull/521
[526]: https://github.com/bevyengine/bevy/pull/526

## Version 0.2.0 (2020-9-19)

### Added

- [Task System for Bevy][384]
  - Replaces rayon with a custom designed task system that consists of several "TaskPools".
  - Exports `IOTaskPool`, `ComputePool`, and `AsyncComputePool` in `bevy_tasks` crate.
- [Parallel queries for distributing work over with the `ParallelIterator` trait.][292]
  - e.g. `query.iter().par_iter(batch_size).for_each(/* ... */)`
- [Added gamepad support using Gilrs][280]
- [Implement WASM support for bevy_winit][503]
- [Create winit canvas under WebAssembly][506]
- [Implement single threaded task scheduler for WebAssembly][496]
- [Support for binary glTF (.glb).][271]
- [Support for `Or` in ECS queries.][358]
- [Added methods `unload()` and `unload_sync()` on `SceneSpawner` for unloading scenes.][339].
- [Custom rodio source for audio.][145]
  - `AudioOuput` is now able to play anything `Decodable`.
- [`Color::hex`][362] for creating `Color` from string hex values.
  - Accepts the forms RGB, RGBA, RRGGBB, and RRGGBBAA.
- [`Color::rgb_u8` and `Color::rgba_u8`.][381]
- [Added `bevy_render::pass::ClearColor` to prelude.][396]
- [`SpriteResizeMode` may choose how `Sprite` resizing should be handled. `Automatic` by default.][430]
- [Added methods on `Input<T>`][428] for iterator access to keys.
  - `get_pressed()`, `get_just_pressed()`, `get_just_released()`
- [Derived `Copy` for `MouseScrollUnit`.][270]
- [Derived `Clone` for UI component bundles.][390]
- [Some examples of documentation][338]
- [Update docs for Updated, Changed and Mutated][451]
- Tips for faster builds on macOS: [#312][312], [#314][314], [#433][433]
- Added and documented cargo features
  - [Created document `docs/cargo_features.md`.][249]
  - [Added features for x11 and wayland display servers.][249]
  - [and added a feature to disable libloading.][363] (helpful for WASM support)
- Added more instructions for Linux dependencies
  - [Arch / Manjaro][275], [NixOS][290], [Ubuntu][463] and [Solus][331]
- [Provide shell.nix for easier compiling with nix-shell][491]
- [Add `AppBuilder::add_startup_stage_|before/after`][505]

### Changed

- [Transform rewrite][374]
- [Use generational entity ids and other optimizations][504]
- [Optimize transform systems to only run on changes.][417]
- [Send an AssetEvent when modifying using `get_id_mut`][323]
- [Rename `Assets::get_id_mut` -> `Assets::get_with_id_mut`][332]
- [Support multiline text in `DrawableText`][183]
- [iOS: use shaderc-rs for glsl to spirv compilation][324]
- [Changed the default node size to Auto instead of Undefined to match the Stretch implementation.][304]
- [Load assets from root path when loading directly][478]
- [Add `render` feature][485], which makes the entire render pipeline optional.

### Fixed

- [Properly track added and removed RenderResources in RenderResourcesNode.][361]
  - Fixes issues where entities vanished or changed color when new entities were spawned/despawned.
- [Fixed sprite clipping at same depth][385]
  - Transparent sprites should no longer clip.
- [Check asset path existence][345]
- [Fixed deadlock in hot asset reloading][376]
- [Fixed hot asset reloading on Windows][394]
- [Allow glTFs to be loaded that don't have uvs and normals][406]
- [Fixed archetypes_generation being incorrectly updated for systems][383]
- [Remove child from parent when it is despawned][386]
- [Initialize App.schedule systems when running the app][444]
- [Fix missing asset info path for synchronous loading][486]
- [fix font atlas overflow][495]
- [do not assume font handle is present in assets][490]

### Internal Improvements

- Many improvements to Bevy's CI [#325][325], [#349][349], [#357][357], [#373][373], [#423][423]

[145]: https://github.com/bevyengine/bevy/pull/145
[183]: https://github.com/bevyengine/bevy/pull/183
[249]: https://github.com/bevyengine/bevy/pull/249
[270]: https://github.com/bevyengine/bevy/pull/270
[271]: https://github.com/bevyengine/bevy/pull/271
[275]: https://github.com/bevyengine/bevy/pull/275
[280]: https://github.com/bevyengine/bevy/pull/280
[290]: https://github.com/bevyengine/bevy/pull/290
[292]: https://github.com/bevyengine/bevy/pull/292
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
[374]: https://github.com/bevyengine/bevy/pull/374
[376]: https://github.com/bevyengine/bevy/pull/376
[381]: https://github.com/bevyengine/bevy/pull/381
[383]: https://github.com/bevyengine/bevy/pull/383
[384]: https://github.com/bevyengine/bevy/pull/384
[385]: https://github.com/bevyengine/bevy/pull/385
[386]: https://github.com/bevyengine/bevy/pull/386
[390]: https://github.com/bevyengine/bevy/pull/390
[394]: https://github.com/bevyengine/bevy/pull/394
[396]: https://github.com/bevyengine/bevy/pull/396
[339]: https://github.com/bevyengine/bevy/pull/339
[406]: https://github.com/bevyengine/bevy/pull/406
[417]: https://github.com/bevyengine/bevy/pull/417
[423]: https://github.com/bevyengine/bevy/pull/423
[428]: https://github.com/bevyengine/bevy/pull/428
[430]: https://github.com/bevyengine/bevy/pull/430
[433]: https://github.com/bevyengine/bevy/pull/433
[444]: https://github.com/bevyengine/bevy/pull/444
[451]: https://github.com/bevyengine/bevy/pull/451
[463]: https://github.com/bevyengine/bevy/pull/463
[478]: https://github.com/bevyengine/bevy/pull/478
[485]: https://github.com/bevyengine/bevy/pull/485
[486]: https://github.com/bevyengine/bevy/pull/486
[490]: https://github.com/bevyengine/bevy/pull/490
[491]: https://github.com/bevyengine/bevy/pull/491
[495]: https://github.com/bevyengine/bevy/pull/495
[496]: https://github.com/bevyengine/bevy/pull/496
[503]: https://github.com/bevyengine/bevy/pull/503
[504]: https://github.com/bevyengine/bevy/pull/504
[505]: https://github.com/bevyengine/bevy/pull/505
[506]: https://github.com/bevyengine/bevy/pull/506

## Version 0.1.3 (2020-8-22)

## Version 0.1.2 (2020-8-10)

## Version 0.1.1 (2020-8-10)

## Version 0.1.0 (2020-8-10)
