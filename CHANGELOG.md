<!-- MD024 - We want repeated headings in a changelog file -->
<!-- markdownlint-disable-file MD024 -->

# Changelog

While we try to keep the `Unreleased` changes updated, it is often behind and does not include
all merged pull requests. To see a list of all changes since the latest release, you may compare
current changes on git with [previous release tags][git_tag_comparison].

[git_tag_comparison]: https://github.com/bevyengine/bevy/compare/v0.4.0...main

## Unreleased

### Added

- [PBR Rendering][1554]
- [PBR Textures][1632]
- [HIDPI Text][1132]
- [Rich text][1245]
- [Wireframe Rendering Pipeline][562]
- [Render Layers][1209]
- [Add Sprite Flipping][1407]
- [OrthographicProjection scaling mode + camera bundle refactoring][400]
- [3D OrthographicProjection improvements + new example][1361]
- [Flexible camera bindings][1689]
- [Render text in 2D scenes][1122]
- [Text2d render quality][1171]
- [System sets and run criteria v2][1675]
- [System sets and parallel executor v2][1144]
- [Many-to-many system labels][1576]
- [Non-string labels (#1423 continued)][1473]
- [Make EventReader a SystemParam][1244]
- [Add EventWriter][1575]
- [Reliable change detection][1471]
- [Redo State architecture][1424]
- [Query::get_unique][1263]
- [gltf: load normal and occlusion as linear textures][1762]
- [Add separate brightness field to AmbientLight][1605]
- [world coords to screen space][1258]
- [Experimental Frustum Culling (for Sprites)][1492]
- [Enable wgpu device limits][1544]
- [bevy_render: add torus and capsule shape][1223]
- [New mesh attribute: color][1194]
- [Minimal change to support instanced rendering][1262]
- [Add support for reading from mapped buffers][1274]
- [Texture atlas format and conversion][1365]
- [enable wgpu device features][547]
- [Subpixel text positioning][1196]
- [make more information available from loaded GLTF model][1020]
- [use Name on node when loading a gltf file][1183]
- [GLTF loader: support mipmap filters][1639]
- [Add support for gltf::Material::unlit][1341]
- [Implement Reflect for tuples up to length 12][1218]
- [Process Asset File Extensions With Multiple Dots][1277]
- [Update Scene Example to Use scn.ron File][1339]
- [3d game example][1252]
- [Add keyboard modifier example (#1656)][1657]
- [Count number of times a repeating Timer wraps around in a tick][1112]
- [recycle `Timer` refactor to duration.sparkles Add `Stopwatch` struct.][1151]
- [add scene instance entity iteration][1058]
- [Make Commands and World apis consistent][1703]
- [Add `insert_children` and `push_children` to EntityMut][1728]
- [Extend AppBuilder api with `add_system_set` and similar methods][1453]
- [add labels and ordering for transform and parent systems in POST_UPDATE stage][1456]
- [Explicit execution order ambiguities API][1469]
- [Resolve (most) internal system ambiguities][1606]
- [Change 'components' to 'bundles' where it makes sense semantically][1257]
- [add `Flags<T>` as a query to get flags of component][1172]
- [Rename add_resource to insert_resource][1356]
- [Update init_resource to not overwrite][1349]
- [Enable dynamic mutable access to component data][1284]
- [Get rid of ChangedRes][1313]
- [impl SystemParam for Option<Res<T>> / Option<ResMut<T>>][1494]
- [Add Window Resize Constraints][1409]
- [Add basic file drag and drop support][1096]
- [Modify Derive to allow unit structs for RenderResources.][1089]
- [bevy_render: load .spv assets][1104]
- [Expose wgpu backend in WgpuOptions and allow it to be configured from the environment][1042]
- [updates on diagnostics (log + new diagnostics)][1085]
- [enable change detection for labels][1155]
- [Name component with fast comparisons][1109]
- [Support for !Send tasks][1216]
- [Add missing spawn_local method to Scope in the single threaded executor case][1266]
- [Add bmp as a supported texture format][1081]
- [Add an alternative winit runner that can be started when not on the main thread][1063]
- [Added use_dpi setting to WindowDescriptor][1131]
- [Implement Copy for ElementState][1154]
- [Mutable mesh accessors: indices_mut and attribute_mut][1164]
- [Add support for OTF fonts][1200]
- [Add `from_xyz` to `Transform`][1212]
- [Adding copy_texture_to_buffer and copy_texture_to_texture][1236]
- [Added `set_minimized` and `set_position` to `Window`][1292]
- [Example for 2D Frustum Culling][1503]
- [Add remove resource to commands][1478]

### Changed

- [Bevy ECS V2][1525]
- [Fix Reflect serialization of tuple structs][1366]
- [color spaces and representation][1572]
- [Make vertex buffers optional][1485]
- [add to lower case to make asset loading case insensitive][1427]
- [Replace right/up/forward and counter parts with local_x/local_y and local_z][1476]
- [Use valid keys to initialize AHasher in FixedState][1268]
- [Change Name to take Into<String> instead of String][1283]
- [Update to wgpu-rs 0.7][542]
- [Update glam to 0.13.0.][1550]
- [use std clamp instead of Bevy's][1644]
- [Make Reflect impls unsafe (Reflect::any must return `self`)][1679]

### Fixed

- [convert grayscale images to rgb][1524]
- [Glb textures should use bevy_render to load images][1454]
- [Don't panic on error when loading assets][1286]
- [Prevent ImageBundles from causing constant layout recalculations][1299]
- [do not check for focus until cursor position has been set][1070]
- [Fix lock order to remove the chance of deadlock][1121]
- [Prevent double panic in the Drop of TaksPoolInner][1064]
- [Ignore events when receiving unknown WindowId][1072]
- [Fix potential bug when using multiple lights.][1055]
- [remove panics when mixing UI and non UI entities in hierarchy][1180]
- [fix label to load gltf scene][1204]
- [fix repeated gamepad events][1221]
- [Fix iOS touch location][1224]
- [Don't panic if there's no index buffer and call draw][1229]
- [Fix Bug in Asset Server Error Message Formatter][1340]
- [add_stage now checks Stage existence][1346]
- [Fix Un-Renamed add_resource Compile Error][1357]
- [Fix Interaction not resetting to None sometimes][1315]
- [Fix regression causing "flipped" sprites to be invisible][1399]
- [revert default vsync mode to Fifo][1416]
- [Fix missing paths in ECS SystemParam derive macro][1434]
- [Fix staging buffer required size calculation (fixes #1056)][1509]


[400]: https://github.com/bevyengine/bevy/pull/400
[542]: https://github.com/bevyengine/bevy/pull/542
[547]: https://github.com/bevyengine/bevy/pull/547
[562]: https://github.com/bevyengine/bevy/pull/562
[1020]: https://github.com/bevyengine/bevy/pull/1020
[1042]: https://github.com/bevyengine/bevy/pull/1042
[1055]: https://github.com/bevyengine/bevy/pull/1055
[1058]: https://github.com/bevyengine/bevy/pull/1058
[1063]: https://github.com/bevyengine/bevy/pull/1063
[1064]: https://github.com/bevyengine/bevy/pull/1064
[1070]: https://github.com/bevyengine/bevy/pull/1070
[1072]: https://github.com/bevyengine/bevy/pull/1072
[1081]: https://github.com/bevyengine/bevy/pull/1081
[1085]: https://github.com/bevyengine/bevy/pull/1085
[1089]: https://github.com/bevyengine/bevy/pull/1089
[1096]: https://github.com/bevyengine/bevy/pull/1096
[1104]: https://github.com/bevyengine/bevy/pull/1104
[1109]: https://github.com/bevyengine/bevy/pull/1109
[1112]: https://github.com/bevyengine/bevy/pull/1112
[1121]: https://github.com/bevyengine/bevy/pull/1121
[1122]: https://github.com/bevyengine/bevy/pull/1122
[1131]: https://github.com/bevyengine/bevy/pull/1131
[1132]: https://github.com/bevyengine/bevy/pull/1132
[1144]: https://github.com/bevyengine/bevy/pull/1144
[1151]: https://github.com/bevyengine/bevy/pull/1151
[1154]: https://github.com/bevyengine/bevy/pull/1154
[1155]: https://github.com/bevyengine/bevy/pull/1155
[1164]: https://github.com/bevyengine/bevy/pull/1164
[1171]: https://github.com/bevyengine/bevy/pull/1171
[1172]: https://github.com/bevyengine/bevy/pull/1172
[1180]: https://github.com/bevyengine/bevy/pull/1180
[1183]: https://github.com/bevyengine/bevy/pull/1183
[1194]: https://github.com/bevyengine/bevy/pull/1194
[1196]: https://github.com/bevyengine/bevy/pull/1196
[1200]: https://github.com/bevyengine/bevy/pull/1200
[1204]: https://github.com/bevyengine/bevy/pull/1204
[1209]: https://github.com/bevyengine/bevy/pull/1209
[1212]: https://github.com/bevyengine/bevy/pull/1212
[1216]: https://github.com/bevyengine/bevy/pull/1216
[1218]: https://github.com/bevyengine/bevy/pull/1218
[1221]: https://github.com/bevyengine/bevy/pull/1221
[1223]: https://github.com/bevyengine/bevy/pull/1223
[1224]: https://github.com/bevyengine/bevy/pull/1224
[1229]: https://github.com/bevyengine/bevy/pull/1229
[1236]: https://github.com/bevyengine/bevy/pull/1236
[1244]: https://github.com/bevyengine/bevy/pull/1244
[1245]: https://github.com/bevyengine/bevy/pull/1245
[1252]: https://github.com/bevyengine/bevy/pull/1252
[1257]: https://github.com/bevyengine/bevy/pull/1257
[1258]: https://github.com/bevyengine/bevy/pull/1258
[1262]: https://github.com/bevyengine/bevy/pull/1262
[1263]: https://github.com/bevyengine/bevy/pull/1263
[1266]: https://github.com/bevyengine/bevy/pull/1266
[1268]: https://github.com/bevyengine/bevy/pull/1268
[1274]: https://github.com/bevyengine/bevy/pull/1274
[1277]: https://github.com/bevyengine/bevy/pull/1277
[1283]: https://github.com/bevyengine/bevy/pull/1283
[1284]: https://github.com/bevyengine/bevy/pull/1284
[1286]: https://github.com/bevyengine/bevy/pull/1286
[1292]: https://github.com/bevyengine/bevy/pull/1292
[1299]: https://github.com/bevyengine/bevy/pull/1299
[1313]: https://github.com/bevyengine/bevy/pull/1313
[1315]: https://github.com/bevyengine/bevy/pull/1315
[1339]: https://github.com/bevyengine/bevy/pull/1339
[1340]: https://github.com/bevyengine/bevy/pull/1340
[1341]: https://github.com/bevyengine/bevy/pull/1341
[1346]: https://github.com/bevyengine/bevy/pull/1346
[1349]: https://github.com/bevyengine/bevy/pull/1349
[1356]: https://github.com/bevyengine/bevy/pull/1356
[1357]: https://github.com/bevyengine/bevy/pull/1357
[1361]: https://github.com/bevyengine/bevy/pull/1361
[1365]: https://github.com/bevyengine/bevy/pull/1365
[1366]: https://github.com/bevyengine/bevy/pull/1366
[1399]: https://github.com/bevyengine/bevy/pull/1399
[1407]: https://github.com/bevyengine/bevy/pull/1407
[1409]: https://github.com/bevyengine/bevy/pull/1409
[1416]: https://github.com/bevyengine/bevy/pull/1416
[1424]: https://github.com/bevyengine/bevy/pull/1424
[1427]: https://github.com/bevyengine/bevy/pull/1427
[1434]: https://github.com/bevyengine/bevy/pull/1434
[1453]: https://github.com/bevyengine/bevy/pull/1453
[1454]: https://github.com/bevyengine/bevy/pull/1454
[1456]: https://github.com/bevyengine/bevy/pull/1456
[1469]: https://github.com/bevyengine/bevy/pull/1469
[1471]: https://github.com/bevyengine/bevy/pull/1471
[1473]: https://github.com/bevyengine/bevy/pull/1473
[1476]: https://github.com/bevyengine/bevy/pull/1476
[1478]: https://github.com/bevyengine/bevy/pull/1478
[1485]: https://github.com/bevyengine/bevy/pull/1485
[1492]: https://github.com/bevyengine/bevy/pull/1492
[1494]: https://github.com/bevyengine/bevy/pull/1494
[1503]: https://github.com/bevyengine/bevy/pull/1503
[1509]: https://github.com/bevyengine/bevy/pull/1509
[1524]: https://github.com/bevyengine/bevy/pull/1524
[1525]: https://github.com/bevyengine/bevy/pull/1525
[1544]: https://github.com/bevyengine/bevy/pull/1544
[1550]: https://github.com/bevyengine/bevy/pull/1550
[1554]: https://github.com/bevyengine/bevy/pull/1554
[1572]: https://github.com/bevyengine/bevy/pull/1572
[1575]: https://github.com/bevyengine/bevy/pull/1575
[1576]: https://github.com/bevyengine/bevy/pull/1576
[1605]: https://github.com/bevyengine/bevy/pull/1605
[1606]: https://github.com/bevyengine/bevy/pull/1606
[1632]: https://github.com/bevyengine/bevy/pull/1632
[1639]: https://github.com/bevyengine/bevy/pull/1639
[1644]: https://github.com/bevyengine/bevy/pull/1644
[1657]: https://github.com/bevyengine/bevy/pull/1657
[1675]: https://github.com/bevyengine/bevy/pull/1675
[1679]: https://github.com/bevyengine/bevy/pull/1679
[1689]: https://github.com/bevyengine/bevy/pull/1689
[1703]: https://github.com/bevyengine/bevy/pull/1703
[1728]: https://github.com/bevyengine/bevy/pull/1728
[1762]: https://github.com/bevyengine/bevy/pull/1762

## Version 0.4.0 (2020-12-19)

### Added

- [add bevymark benchmark example][273]
- [gltf: support camera and fix hierarchy][772]
- [Add tracing spans to schedules, stages, systems][789]
- [add example that represents contributors as bevy icons][801]
- [Add received character][805]
- [Add bevy_dylib to force dynamic linking of bevy][808]
- [Added RenderPass::set_scissor_rect][815]
- [`bevy_log`][836]
  - Adds logging functionality as a Plugin.
  - Changes internal logging to work with the new implementation.
- [cross-platform main function][847]
- [Controllable ambient light color][852]
  - Added a resource to change the current ambient light color for PBR.
- [Added more basic color constants][859]
- [Add box shape][883]
- [Expose an EventId for events][894]
- [System Inputs, Outputs, and Chaining][876]
- [Expose an `EventId` for events][894]
- [Added `set_cursor_position` to `Window`][917]
- [Added new Bevy reflection system][926]
  - Replaces the properties system
- [Add support for Apple Silicon][928]
- [Live reloading of shaders][937]
- [Store mouse cursor position in Window][940]
- [Add removal_detection example][945]
- [Additional vertex attribute value types][946]
- [Added WindowFocused event][956]
- [Tracing chrome span names][979]
- [Allow windows to be maximized][1004]
- [GLTF: load default material][1016]
- [can spawn a scene from a ChildBuilder, or directly set its parent when spawning it][1026]
- [add ability to load `.dds`, `.tga`, and `.jpeg` texture formats][1038]
- [add ability to provide custom a `AssetIo` implementation][1037]

### Changed

- [delegate layout reflection to RenderResourceContext][691]
- [Fall back to remove components one by one when failing to remove a bundle][719]
- [Port hecs derive macro improvements][761]
- [Use glyph_brush_layout and add text alignment support][765]
- [upgrade glam and hexasphere][791]
- [Flexible ECS Params][798]
- [Make Timer.tick return &Self][820]
- [FileAssetIo includes full path on error][821]
- [Removed ECS query APIs that could easily violate safety from the public interface][829]
- [Changed Query filter API to be easier to understand][834]
- [bevy_render: delegate buffer aligning to render_resource_context][842]
- [wasm32: non-spirv shader specialization][843]
- [Renamed XComponents to XBundle][863]
- [Check for conflicting system resource parameters][864]
- [Tweaks to TextureAtlasBuilder.finish()][887]
- [do not spend time drawing text with is_visible = false][893]
- [Extend the Texture asset type to support 3D data][903]
- [Breaking changes to timer API][914]
  - Created getters and setters rather than exposing struct members.
- [Removed timer auto-ticking system][931]
  - Added an example of how to tick timers manually.
- [When a task scope produces <= 1 task to run, run it on the calling thread immediately][932]
- [Breaking changes to Time API][934]
  - Created getters to get `Time` state and made members private.
  - Modifying `Time`'s values directly is no longer possible outside of bevy.
- [Use `mailbox` instead of `fifo` for vsync on supported systems][920]
- [switch winit size to logical to be dpi independent][947]
- [Change bevy_input::Touch API to match similar APIs][952]
- [Run parent-update and transform-propagation during the "post-startup" stage (instead of "startup")][955]
- [Renderer Optimization Round 1][958]
- [Change`TextureAtlasBuilder` into expected Builder conventions][969]
- [Optimize Text rendering / SharedBuffers][972]
- [hidpi swap chains][973]
- [optimize asset gpu data transfer][987]
- [naming coherence for cameras][995]
- [Schedule v2][1021]
- [Use shaderc for aarch64-apple-darwin][1027]
- [update `Window`'s `width` & `height` methods to return `f32`][1033]
- [Break out Visible component from Draw][1034]
  - Users setting `Draw::is_visible` or `Draw::is_transparent` should now set `Visible::is_visible` and `Visible::is_transparent`
- [`winit` upgraded from version 0.23 to version 0.24][1043]
- [set is_transparent to true by default for UI bundles][1071]

### Fixed

- [Fixed typos in KeyCode identifiers][857]
- [Remove redundant texture copies in TextureCopyNode][871]
- [Fix a deadlock that can occur when using scope() on ComputeTaskPool from within a system][892]
- [Don't draw text that isn't visible][893]
- [Use `instant::Instant` for WASM compatibility][895]
- [Fix pixel format conversion in bevy_gltf][897]
- [Fixed duplicated children when spawning a Scene][904]
- [Corrected behaviour of the UI depth system][905]
- [Allow despawning of hierarchies in threadlocal systems][908]
- [Fix `RenderResources` index slicing][948]
- [Run parent-update and transform-propagation during the "post-startup" stage][955]
- [Fix collision detection by comparing abs() penetration depth][966]
- [deal with rounding issue when creating the swap chain][997]
- [only update components for entities in map][1023]
- [Don't panic when attempting to set shader defs from an asset that hasn't loaded yet][1035]

[273]: https://github.com/bevyengine/bevy/pull/273
[691]: https://github.com/bevyengine/bevy/pull/691
[719]: https://github.com/bevyengine/bevy/pull/719
[761]: https://github.com/bevyengine/bevy/pull/761
[761]: https://github.com/bevyengine/bevy/pull/761
[765]: https://github.com/bevyengine/bevy/pull/765
[772]: https://github.com/bevyengine/bevy/pull/772
[772]: https://github.com/bevyengine/bevy/pull/772
[789]: https://github.com/bevyengine/bevy/pull/789
[791]: https://github.com/bevyengine/bevy/pull/791
[798]: https://github.com/bevyengine/bevy/pull/798
[801]: https://github.com/bevyengine/bevy/pull/801
[801]: https://github.com/bevyengine/bevy/pull/801
[805]: https://github.com/bevyengine/bevy/pull/805
[808]: https://github.com/bevyengine/bevy/pull/808
[815]: https://github.com/bevyengine/bevy/pull/815
[820]: https://github.com/bevyengine/bevy/pull/820
[821]: https://github.com/bevyengine/bevy/pull/821
[821]: https://github.com/bevyengine/bevy/pull/821
[829]: https://github.com/bevyengine/bevy/pull/829
[829]: https://github.com/bevyengine/bevy/pull/829
[834]: https://github.com/bevyengine/bevy/pull/834
[834]: https://github.com/bevyengine/bevy/pull/834
[836]: https://github.com/bevyengine/bevy/pull/836
[836]: https://github.com/bevyengine/bevy/pull/836
[842]: https://github.com/bevyengine/bevy/pull/842
[843]: https://github.com/bevyengine/bevy/pull/843
[847]: https://github.com/bevyengine/bevy/pull/847
[852]: https://github.com/bevyengine/bevy/pull/852
[852]: https://github.com/bevyengine/bevy/pull/852
[857]: https://github.com/bevyengine/bevy/pull/857
[857]: https://github.com/bevyengine/bevy/pull/857
[859]: https://github.com/bevyengine/bevy/pull/859
[859]: https://github.com/bevyengine/bevy/pull/859
[863]: https://github.com/bevyengine/bevy/pull/863
[864]: https://github.com/bevyengine/bevy/pull/864
[871]: https://github.com/bevyengine/bevy/pull/871
[876]: https://github.com/bevyengine/bevy/pull/876
[876]: https://github.com/bevyengine/bevy/pull/876
[883]: https://github.com/bevyengine/bevy/pull/883
[887]: https://github.com/bevyengine/bevy/pull/887
[892]: https://github.com/bevyengine/bevy/pull/892
[893]: https://github.com/bevyengine/bevy/pull/893
[893]: https://github.com/bevyengine/bevy/pull/893
[893]: https://github.com/bevyengine/bevy/pull/893
[894]: https://github.com/bevyengine/bevy/pull/894
[894]: https://github.com/bevyengine/bevy/pull/894
[894]: https://github.com/bevyengine/bevy/pull/894
[895]: https://github.com/bevyengine/bevy/pull/895
[895]: https://github.com/bevyengine/bevy/pull/895
[897]: https://github.com/bevyengine/bevy/pull/897
[903]: https://github.com/bevyengine/bevy/pull/903
[904]: https://github.com/bevyengine/bevy/pull/904
[904]: https://github.com/bevyengine/bevy/pull/904
[905]: https://github.com/bevyengine/bevy/pull/905
[905]: https://github.com/bevyengine/bevy/pull/905
[908]: https://github.com/bevyengine/bevy/pull/908
[914]: https://github.com/bevyengine/bevy/pull/914
[914]: https://github.com/bevyengine/bevy/pull/914
[917]: https://github.com/bevyengine/bevy/pull/917
[917]: https://github.com/bevyengine/bevy/pull/917
[920]: https://github.com/bevyengine/bevy/pull/920
[920]: https://github.com/bevyengine/bevy/pull/920
[926]: https://github.com/bevyengine/bevy/pull/926
[926]: https://github.com/bevyengine/bevy/pull/926
[928]: https://github.com/bevyengine/bevy/pull/928
[928]: https://github.com/bevyengine/bevy/pull/928
[931]: https://github.com/bevyengine/bevy/pull/931
[931]: https://github.com/bevyengine/bevy/pull/931
[932]: https://github.com/bevyengine/bevy/pull/932
[934]: https://github.com/bevyengine/bevy/pull/934
[934]: https://github.com/bevyengine/bevy/pull/934
[937]: https://github.com/bevyengine/bevy/pull/937
[940]: https://github.com/bevyengine/bevy/pull/940
[945]: https://github.com/bevyengine/bevy/pull/945
[945]: https://github.com/bevyengine/bevy/pull/945
[946]: https://github.com/bevyengine/bevy/pull/946
[947]: https://github.com/bevyengine/bevy/pull/947
[948]: https://github.com/bevyengine/bevy/pull/948
[952]: https://github.com/bevyengine/bevy/pull/952
[955]: https://github.com/bevyengine/bevy/pull/955
[955]: https://github.com/bevyengine/bevy/pull/955
[955]: https://github.com/bevyengine/bevy/pull/955
[956]: https://github.com/bevyengine/bevy/pull/956
[958]: https://github.com/bevyengine/bevy/pull/958
[966]: https://github.com/bevyengine/bevy/pull/966
[969]: https://github.com/bevyengine/bevy/pull/969
[972]: https://github.com/bevyengine/bevy/pull/972
[973]: https://github.com/bevyengine/bevy/pull/973
[979]: https://github.com/bevyengine/bevy/pull/979
[987]: https://github.com/bevyengine/bevy/pull/987
[995]: https://github.com/bevyengine/bevy/pull/995
[997]: https://github.com/bevyengine/bevy/pull/997
[1004]: https://github.com/bevyengine/bevy/pull/1004
[1016]: https://github.com/bevyengine/bevy/pull/1016
[1021]: https://github.com/bevyengine/bevy/pull/1021
[1023]: https://github.com/bevyengine/bevy/pull/1023
[1026]: https://github.com/bevyengine/bevy/pull/1026
[1027]: https://github.com/bevyengine/bevy/pull/1027
[1033]: https://github.com/bevyengine/bevy/pull/1033
[1034]: https://github.com/bevyengine/bevy/pull/1034
[1034]: https://github.com/bevyengine/bevy/pull/1034
[1035]: https://github.com/bevyengine/bevy/pull/1035
[1037]: https://github.com/bevyengine/bevy/pull/1037
[1038]: https://github.com/bevyengine/bevy/pull/1038
[1043]: https://github.com/bevyengine/bevy/pull/1043
[1043]: https://github.com/bevyengine/bevy/pull/1043
[1071]: https://github.com/bevyengine/bevy/pull/1071

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
