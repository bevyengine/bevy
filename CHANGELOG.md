<!-- MD024 - We want repeated headings in a changelog file -->
<!-- markdownlint-disable-file MD024 -->

# Changelog

While we try to keep the `Unreleased` changes updated, it is often behind and does not include
all merged pull requests. To see a list of all changes since the latest release, you may compare
current changes on git with [previous release tags][git_tag_comparison].

[git_tag_comparison]: https://github.com/bevyengine/bevy/compare/v0.9.0...main

## Version 0.9.0 (2022-11-12)

### Added

- [Bloom][6397]
- [Add FXAA postprocessing][6393]
- [Fix color banding by dithering image before quantization][5264]
- [Plugins own their settings. Rework PluginGroup trait.][6336]
- [Add global time scaling][5752]
- [add globals to mesh view bind group][5409]
- [Add UI scaling][5814]
- [Add FromReflect for Timer][6422]
- [Re-add local bool `has_received_time` in `time_system`][6357]
- [Add default implementation of Serialize and Deserialize to Timer and Stopwatch][6248]
- [add time wrapping to Time][5982]
- [Stopwatch elapsed secs f64][5978]
- [Remaining fn in Timer][5971]
- [Support array / cubemap / cubemap array textures in KTX2][5325]
- [Add methods for silencing system-order ambiguity warnings][6158]
- [bevy_dynamic_plugin: make it possible to handle loading errors][6437]
- [can get the settings of a plugin from the app][6372]
- [Use plugin setup for resource only used at setup time][6360]
- [Add `TimeUpdateStrategy` resource for manual `Time` updating][6159]
- [dynamic scene builder][6227]
- [Create a scene from a dynamic scene][6229]
- [Scene example: write file in a task][5952]
- [Add writing of scene data to Scene example][5949]
- [can clone a scene][5855]
- [Add "end of main pass post processing" render graph node][6468]
- [Add `Camera::viewport_to_world`][6126]
- [Sprite: allow using a sub-region (Rect) of the image][6014]
- [Add missing type registrations for bevy_math types][5758]
- [Add `serialize` feature to `bevy_core`][6423]
- [add serialize feature to bevy_transform][6379]
- [Add associated constant `IDENTITY` to `Transform` and friends.][5340]
- [bevy_reflect: Add `Reflect::into_reflect`][6502]
- [Add reflect_owned][6494]
- [`Reflect` for `Tonemapping` and `ClusterConfig`][6488]
- [add `ReflectDefault` to std types][6429]
- [Add FromReflect for Visibility][6410]
- [Register `RenderLayers` type in `CameraPlugin`][6308]
- [Enable Constructing ReflectComponent/Resource][6257]
- [Support multiple `#[reflect]`/`#[reflect_value]` + improve error messages][6237]
- [Reflect Default for GlobalTransform][6200]
- [Impl Reflect for PathBuf and OsString][6193]
- [Reflect Default for `ComputedVisibility` and `Handle<T>`][6187]
- [Register `Wireframe` type][6152]
- [Derive `FromReflect` for `Transform` and `GlobalTransform`][6015]
- [Make arrays behave like lists in reflection][5987]
- [Implement `Debug` for dynamic types][5948]
- [Implemented `Reflect` for all the ranges][5806]
- [Add `pop` method for `List` trait.][5797]
- [bevy_reflect: `GetTypeRegistration` for `SmallVec<T>`][5782]
- [register missing reflect types][5747]
- [bevy_reflect: Get owned fields][5728]
- [bevy_reflect: Add `FromReflect` to the prelude][5720]
- [implement `Reflect` for `Input<T>`, some misc improvements to reflect value derive][5676]
- [register `Cow<'static, str>` for reflection][5664]
- [bevy_reflect: Relax bounds on `Option<T>`][5658]
- [remove `ReflectMut` in favor of `Mut<dyn Reflect>`][5630]
- [add some info from `ReflectPathError` to the error messages][5626]
- [Added reflect/from reflect impls for NonZero integer types][5556]
- [bevy_reflect: Update enum derives][5473]
- [Add `reflect(skip_serializing)` which retains reflection but disables automatic serialization][5250]
- [bevy_reflect: Reflect enums][4761]
- [Disabling default features support in bevy_ecs, bevy_reflect and bevy][5993]
- [expose window alpha mode][6331]
- [Make bevy_window and bevy_input events serializable][6180]
- [Add window resizing example][5813]
- [feat: add GamepadInfo, expose gamepad names][6342]
- [Derive `Reflect` + `FromReflect` for input types][6232]
- [Make TouchInput and ForceTouch serializable][6191]
- [Add a Gamepad Viewer tool to examples][6074]
- [Derived `Copy` trait for `bevy_input` events, `Serialize`/`Deserialize` for events in `bevy_input` and `bevy_windows`, `PartialEq` for events in both, and `Eq` where possible in both.][6023]
- [Support for additional gamepad buttons and axis][5853]
- [Added keyboard scan input event][5495]
- [Add `set_parent` and `remove_parent` to `EntityCommands`][6189]
- [Add methods to `Query<&Children>` and `Query<&Parent>` to iterate over descendants and ancestors][6185]
- [Add `is_finished` to `Task<T>`][6444]
- [Expose mint feature in bevy_math/glam][5857]
- [Utility methods for Val][6134]
- [Register missing bevy_text types][6029]
- [Add additional constructors for `UiRect` to specify values for specific fields][5988]
- [Add AUTO and UNDEFINED const constructors for `Size`][5761]
- [Add Exponential Moving Average into diagnostics][4992]
- [Add `send_event` and friends to `WorldCell`][6515]
- [Add a method for accessing the width of a `Table`][6249]
- [Add iter_entities to World #6228][6242]
- [Adding Debug implementations for App, Stage, Schedule, Query, QueryState, etc.][6214]
- [Add a method for mapping `Mut<T>` -> `Mut<U>`][6199]
- [implemented #[bundle(ignore)]][6123]
- [Allow access to non-send resource through `World::resource_scope`][6113]
- [Add get_entity to Commands][5854]
- [Added the ability to get or set the last change tick of a system.][5838]
- [Add a module for common system `chain`/`pipe` adapters][5776]
- [SystemParam for the name of the system you are currently in][5731]
- [Warning message for missing events][5730]
- [Add a change detection bypass and manual control over change ticks][5635]
- [Add into_world_mut to EntityMut][5586]
- [Add `FromWorld` bound to `T` in `Local<T>`][5481]
- [Add `From<EntityMut>` for EntityRef (fixes #5459)][5461]
- [Implement IntoIterator for ECS wrapper types.][5096]
- [add `Res::clone`][4109]
- [Add CameraRenderGraph::set][6470]
- [Use wgsl saturate][6318]
- [Add mutating `toggle` method to `Visibility` component][6268]
- [Add globals struct to mesh2d][6222]
- [add support for .comp glsl shaders][6084]
- [Implement `IntoIterator` for `&Extract<P>`][6025]
- [add Debug, Copy, Clone derives to Circle][6009]
- [Add TextureFormat::Rg16Unorm support for Image and derive Resource for SpecializedComputePipelines][5991]
- [Add `bevy_render::texture::ImageSettings` to prelude][5566]
- [Add `Projection` component to prelude.][5557]
- [Expose `Image` conversion functions (fixes #5452)][5527]
- [Macro for Loading Internal Binary Assets][6478]
- [Add `From<String>` for `AssetPath<'a>`][6337]
- [Add Eq & PartialEq to AssetPath][6274]
- [add `ReflectAsset` and `ReflectHandle`][5923]
- [Add warning when using load_folder on web][5827]
- [Expose rodio's Source and Sample traits in bevy_audio][6374]
- [Add a way to toggle `AudioSink`][6321]

### Changed

- [separate tonemapping and upscaling passes][3425]
- [Rework ViewTarget to better support post processing][6415]
- [bevy_reflect: Improve serialization format even more][5723]
- [bevy_reflect: Binary formats][6140]
- [Unique plugins][6411]
- [Support arbitrary RenderTarget texture formats][6380]
- [Make `Resource` trait opt-in, requiring `#[derive(Resource)]` V2][5577]
- [Replace `WorldQueryGats` trait with actual gats][6319]
- [Change UI coordinate system to have origin at top left corner][6000]
- [Move the cursor's origin back to the bottom-left][6533]
- [Add z-index support with a predictable UI stack][5877]
- [TaskPool Panic Handling][6443]
- [Implement `Bundle` for `Component`. Use `Bundle` tuples for insertion][2975]
- [Spawn now takes a Bundle][6054]
- [make `WorldQuery` very flat][5205]
- [Accept Bundles for insert and remove. Deprecate insert/remove_bundle][6039]
- [Exclusive Systems Now Implement `System`. Flexible Exclusive System Params][6083]
- [bevy_scene: Serialize entities to map][6416]
- [bevy_scene: Stabilize entity order in `DynamicSceneBuilder`][6382]
- [bevy_scene: Replace root list with struct][6354]
- [bevy_scene: Use map for scene `components`][6345]
- [Start running systems while prepare_systems is running][4919]
- [Extract Resources into their own dedicated storage][4809]
- [get proper texture format after the renderer is initialized, fix #3897][5413]
- [Add getters and setters for `InputAxis` and `ButtonSettings`][6088]
- [Clean up Fetch code][4800]
- [Nested spawns on scope][4466]
- [Skip empty archetypes and tables when iterating over queries][4724]
- [Increase the `MAX_DIRECTIONAL_LIGHTS` from 1 to 10][6066]
- [bevy_pbr: Normalize skinned normals][6543]
- [remove mandatory mesh attributes][6127]
- [Rename `play` to `start` and add new `play` method that won't overwrite the existing animation if it's already playing][6350]
- [Replace the `bool` argument of `Timer` with `TimerMode`][6247]
- [improve panic messages for add_system_to_stage and add_system_set_to_stage][5847]
- [Use default serde impls for Entity][6194]
- [scenes: simplify return type of iter_instance_entities][5994]
- [Consistently use `PI` to specify angles in examples.][5825]
- [Remove `Transform::apply_non_uniform_scale`][6133]
- [Rename `Transform::mul_vec3` to `transform_point` and improve docs][6132]
- [make `register` on `TypeRegistry` idempotent][6487]
- [do not set cursor grab on window creation if not asked for][6381]
- [Make `raw_window_handle` field in `Window` and `ExtractedWindow` an `Option`.][6114]
- [Support monitor selection for all window modes.][5878]
- [`Gamepad` type is `Copy`; do not require / return references to it in `Gamepads` API][5296]
- [Update tracing-chrome to 0.6.0][6398]
- [Update to ron 0.8][5864]
- [Update clap requirement from 3.2 to 4.0][6303]
- [Update glam 0.22, hexasphere 8.0, encase 0.4][6427]
- [Update `wgpu` to 0.14.0, `naga` to `0.10.0`, `winit` to 0.27.4, `raw-window-handle` to 0.5.0, `ndk` to 0.7][6218]
- [Update to notify 5.0 stable][5865]
- [Update rodio requirement from 0.15 to 0.16][6020]
- [remove copyless][6100]
- [Mark `Task` as `#[must_use]`][6068]
- [Swap out num_cpus for std::thread::available_parallelism][4970]
- [Cleaning up NodeBundle, and some slight UI module re-organization][6473]
- [Make the default background color of `NodeBundle` transparent][6211]
- [Rename `UiColor`  to `BackgroundColor`][6087]
- [changed diagnostics from seconds to milliseconds][5554]
- [Remove unnecesary branches/panics from Query accesses][6461]
- [`debug_checked_unwrap` should track its caller][6452]
- [Speed up `Query::get_many` and add benchmarks][6400]
- [Rename system chaining to system piping][6230]
- [[Fixes #6059] ``Entity``'s “ID” should be named “index” instead][6107]
- [`Query` filter types must be `ReadOnlyWorldQuery`][6008]
- [Remove ambiguity sets][5916]
- [relax `Sized` bounds around change detection types][5917]
- [Remove ExactSizeIterator from QueryCombinationIter][5895]
- [Remove Sync bound from Command][5871]
- [Make most `Entity` methods `const`][5688]
- [Remove `insert_resource_with_id`][5608]
- [Avoid making `Fetch`s `Clone`][5593]
- [Remove `Sync` bound from `Local`][5483]
- [Replace `many_for_each_mut` with `iter_many_mut`.][5402]
- [bevy_ecs: Use 32-bit entity ID cursor on platforms without AtomicI64][4452]
- [Specialize UI pipeline on "hdr-ness"][6459]
- [Allow passing `glam` vector types as vertex attributes][6442]
- [Add multi draw indirect draw calls][6392]
- [Take DirectionalLight's GlobalTransform into account when calculating shadow map volume (not just direction)][6384]
- [Respect mipmap_filter when create ImageDescriptor with linear()/nearest()][6349]
- [use bevy default texture format if the surface is not yet available][6233]
- [log pipeline cache errors earlier][6115]
- [Merge TextureAtlas::from_grid_with_padding into TextureAtlas::from_grid through option arguments][6057]
- [Reconfigure surface on present mode change][6049]
- [Use 3 bits of PipelineKey to store MSAA sample count][5826]
- [Limit FontAtlasSets][5708]
- [Move `sprite::Rect` into `bevy_math`][5686]
- [Make vertex colors work without textures in bevy_sprite][5685]
- [use bevy_default() for texture format in post_processing][5601]
- [don't render completely transparent UI nodes][5537]
- [make TextLayoutInfo a Component][4460]
- [make `Handle::<T>` field id private, and replace with a getter][6176]
- [Remove `AssetServer::watch_for_changes()`][5968]
- [Rename Handle::as_weak() to cast_weak()][5321]
- [Remove `Sync` requirement in `Decodable::Decoder`][5819]

### Fixed

- [Optimize rendering slow-down at high entity counts][5509]
- [bevy_reflect: Fix `DynamicScene` not respecting component registrations during serialization][6288]
- [fixes the types for Vec3 and Quat in scene example to remove WARN from the logs][5751]
- [Fix end-of-animation index OOB][6210]
- [bevy_reflect: Remove unnecessary `Clone` bounds][5783]
- [bevy_reflect: Fix `apply` method for `Option<T>`][5780]
- [Fix outdated and badly formatted docs for `WindowDescriptor::transparent`][6329]
- [disable window pre creation for ios][5883]
- [Remove unnecessary unsafe `Send` and `Sync` impl for `WinitWindows` on wasm.][5863]
- [Fix window centering when scale_factor is not 1.0][5582]
- [fix order of exit/close window systems][5558]
- [bevy_input: Fix process touch event][4352]
- [fix: explicitly specify required version of async-task][6509]
- [Fix `clippy::iter_with_drain`][6485]
- [Use `cbrt()` instead of `powf(1./3.)`][6481]
- [Fix `RemoveChildren` command][6192]
- [Fix inconsistent children removal behavior][6017]
- [tick local executor][6121]
- [Fix panic when the primary window is closed][6545]
- [UI scaling fix][6479]
- [Fix clipping in UI][6351]
- [Fixes scroll example after inverting UI Y axis][6290]
- [Fixes incorrect glyph positioning for text2d][6273]
- [Clean up taffy nodes when UI node entities are removed][5886]
- [Fix unsound `EntityMut::remove_children`. Add `EntityMut::world_scope`][6464]
- [Fix spawning empty bundles][6425]
- [Fix query.to_readonly().get_component_mut() soundness bug][6401]
- [#5817: derive_bundle macro is not hygienic][5835]
- [drop old value in `insert_resource_by_id` if exists][5587]
- [Fix lifetime bound on `From` impl for `NonSendMut` -> `Mut`][5560]
- [Fix `mesh.wgsl` error for meshes without normals][6439]
- [Fix panic when using globals uniform in wasm builds][6460]
- [Resolve most remaining execution-order ambiguities][6341]
- [Call `mesh2d_tangent_local_to_world` with the right arguments][6209]
- [Fixes Camera not being serializable due to missing registrations in core functionality.][6170]
- [fix spot dir nan bug][6167]
- [use alpha mask even when unlit][6047]
- [Ignore `Timeout` errors on Linux AMD & Intel][5957]
- [adjust cluster index for viewport origin][5947]
- [update camera projection if viewport changed][5945]
- [Ensure 2D phase items are sorted before batching][5942]
- [bevy_pbr: Fix incorrect and unnecessary normal-mapping code][5766]
- [Add explicit ordering between `update_frusta` and `camera_system`][5757]
- [bevy_pbr: Fix tangent and normal normalization][5666]
- [Fix shader syntax][5613]
- [Correctly use as_hsla_f32 in `Add<Color>` and `AddAssign<Color>`, fixes #5543][5546]
- [Sync up bevy_sprite and bevy_ui shader View struct][5531]
- [Fix View by adding missing fields present in ViewUniform][5512]
- [Freeing memory held by visible entities vector][3009]
- [Correctly parse labels with '#'][5729]

[6545]: https://github.com/bevyengine/bevy/pull/6545
[6543]: https://github.com/bevyengine/bevy/pull/6543
[6533]: https://github.com/bevyengine/bevy/pull/6533
[6515]: https://github.com/bevyengine/bevy/pull/6515
[6509]: https://github.com/bevyengine/bevy/pull/6509
[6502]: https://github.com/bevyengine/bevy/pull/6502
[6494]: https://github.com/bevyengine/bevy/pull/6494
[6488]: https://github.com/bevyengine/bevy/pull/6488
[6487]: https://github.com/bevyengine/bevy/pull/6487
[6485]: https://github.com/bevyengine/bevy/pull/6485
[6481]: https://github.com/bevyengine/bevy/pull/6481
[6479]: https://github.com/bevyengine/bevy/pull/6479
[6478]: https://github.com/bevyengine/bevy/pull/6478
[6473]: https://github.com/bevyengine/bevy/pull/6473
[6470]: https://github.com/bevyengine/bevy/pull/6470
[6468]: https://github.com/bevyengine/bevy/pull/6468
[6464]: https://github.com/bevyengine/bevy/pull/6464
[6461]: https://github.com/bevyengine/bevy/pull/6461
[6460]: https://github.com/bevyengine/bevy/pull/6460
[6459]: https://github.com/bevyengine/bevy/pull/6459
[6452]: https://github.com/bevyengine/bevy/pull/6452
[6444]: https://github.com/bevyengine/bevy/pull/6444
[6443]: https://github.com/bevyengine/bevy/pull/6443
[6442]: https://github.com/bevyengine/bevy/pull/6442
[6439]: https://github.com/bevyengine/bevy/pull/6439
[6437]: https://github.com/bevyengine/bevy/pull/6437
[6429]: https://github.com/bevyengine/bevy/pull/6429
[6427]: https://github.com/bevyengine/bevy/pull/6427
[6425]: https://github.com/bevyengine/bevy/pull/6425
[6423]: https://github.com/bevyengine/bevy/pull/6423
[6422]: https://github.com/bevyengine/bevy/pull/6422
[6416]: https://github.com/bevyengine/bevy/pull/6416
[6415]: https://github.com/bevyengine/bevy/pull/6415
[6411]: https://github.com/bevyengine/bevy/pull/6411
[6410]: https://github.com/bevyengine/bevy/pull/6410
[6401]: https://github.com/bevyengine/bevy/pull/6401
[6400]: https://github.com/bevyengine/bevy/pull/6400
[6398]: https://github.com/bevyengine/bevy/pull/6398
[6397]: https://github.com/bevyengine/bevy/pull/6397
[6393]: https://github.com/bevyengine/bevy/pull/6393
[6392]: https://github.com/bevyengine/bevy/pull/6392
[6384]: https://github.com/bevyengine/bevy/pull/6384
[6382]: https://github.com/bevyengine/bevy/pull/6382
[6381]: https://github.com/bevyengine/bevy/pull/6381
[6380]: https://github.com/bevyengine/bevy/pull/6380
[6379]: https://github.com/bevyengine/bevy/pull/6379
[6374]: https://github.com/bevyengine/bevy/pull/6374
[6372]: https://github.com/bevyengine/bevy/pull/6372
[6360]: https://github.com/bevyengine/bevy/pull/6360
[6357]: https://github.com/bevyengine/bevy/pull/6357
[6354]: https://github.com/bevyengine/bevy/pull/6354
[6351]: https://github.com/bevyengine/bevy/pull/6351
[6350]: https://github.com/bevyengine/bevy/pull/6350
[6349]: https://github.com/bevyengine/bevy/pull/6349
[6345]: https://github.com/bevyengine/bevy/pull/6345
[6342]: https://github.com/bevyengine/bevy/pull/6342
[6341]: https://github.com/bevyengine/bevy/pull/6341
[6337]: https://github.com/bevyengine/bevy/pull/6337
[6336]: https://github.com/bevyengine/bevy/pull/6336
[6331]: https://github.com/bevyengine/bevy/pull/6331
[6329]: https://github.com/bevyengine/bevy/pull/6329
[6321]: https://github.com/bevyengine/bevy/pull/6321
[6319]: https://github.com/bevyengine/bevy/pull/6319
[6318]: https://github.com/bevyengine/bevy/pull/6318
[6308]: https://github.com/bevyengine/bevy/pull/6308
[6303]: https://github.com/bevyengine/bevy/pull/6303
[6290]: https://github.com/bevyengine/bevy/pull/6290
[6288]: https://github.com/bevyengine/bevy/pull/6288
[6274]: https://github.com/bevyengine/bevy/pull/6274
[6273]: https://github.com/bevyengine/bevy/pull/6273
[6268]: https://github.com/bevyengine/bevy/pull/6268
[6257]: https://github.com/bevyengine/bevy/pull/6257
[6249]: https://github.com/bevyengine/bevy/pull/6249
[6248]: https://github.com/bevyengine/bevy/pull/6248
[6247]: https://github.com/bevyengine/bevy/pull/6247
[6242]: https://github.com/bevyengine/bevy/pull/6242
[6237]: https://github.com/bevyengine/bevy/pull/6237
[6233]: https://github.com/bevyengine/bevy/pull/6233
[6232]: https://github.com/bevyengine/bevy/pull/6232
[6230]: https://github.com/bevyengine/bevy/pull/6230
[6229]: https://github.com/bevyengine/bevy/pull/6229
[6227]: https://github.com/bevyengine/bevy/pull/6227
[6222]: https://github.com/bevyengine/bevy/pull/6222
[6218]: https://github.com/bevyengine/bevy/pull/6218
[6214]: https://github.com/bevyengine/bevy/pull/6214
[6211]: https://github.com/bevyengine/bevy/pull/6211
[6210]: https://github.com/bevyengine/bevy/pull/6210
[6209]: https://github.com/bevyengine/bevy/pull/6209
[6200]: https://github.com/bevyengine/bevy/pull/6200
[6199]: https://github.com/bevyengine/bevy/pull/6199
[6194]: https://github.com/bevyengine/bevy/pull/6194
[6193]: https://github.com/bevyengine/bevy/pull/6193
[6192]: https://github.com/bevyengine/bevy/pull/6192
[6191]: https://github.com/bevyengine/bevy/pull/6191
[6189]: https://github.com/bevyengine/bevy/pull/6189
[6187]: https://github.com/bevyengine/bevy/pull/6187
[6185]: https://github.com/bevyengine/bevy/pull/6185
[6180]: https://github.com/bevyengine/bevy/pull/6180
[6176]: https://github.com/bevyengine/bevy/pull/6176
[6170]: https://github.com/bevyengine/bevy/pull/6170
[6167]: https://github.com/bevyengine/bevy/pull/6167
[6159]: https://github.com/bevyengine/bevy/pull/6159
[6158]: https://github.com/bevyengine/bevy/pull/6158
[6152]: https://github.com/bevyengine/bevy/pull/6152
[6140]: https://github.com/bevyengine/bevy/pull/6140
[6134]: https://github.com/bevyengine/bevy/pull/6134
[6133]: https://github.com/bevyengine/bevy/pull/6133
[6132]: https://github.com/bevyengine/bevy/pull/6132
[6127]: https://github.com/bevyengine/bevy/pull/6127
[6126]: https://github.com/bevyengine/bevy/pull/6126
[6123]: https://github.com/bevyengine/bevy/pull/6123
[6121]: https://github.com/bevyengine/bevy/pull/6121
[6115]: https://github.com/bevyengine/bevy/pull/6115
[6114]: https://github.com/bevyengine/bevy/pull/6114
[6113]: https://github.com/bevyengine/bevy/pull/6113
[6107]: https://github.com/bevyengine/bevy/pull/6107
[6100]: https://github.com/bevyengine/bevy/pull/6100
[6088]: https://github.com/bevyengine/bevy/pull/6088
[6087]: https://github.com/bevyengine/bevy/pull/6087
[6084]: https://github.com/bevyengine/bevy/pull/6084
[6083]: https://github.com/bevyengine/bevy/pull/6083
[6074]: https://github.com/bevyengine/bevy/pull/6074
[6068]: https://github.com/bevyengine/bevy/pull/6068
[6066]: https://github.com/bevyengine/bevy/pull/6066
[6057]: https://github.com/bevyengine/bevy/pull/6057
[6054]: https://github.com/bevyengine/bevy/pull/6054
[6049]: https://github.com/bevyengine/bevy/pull/6049
[6047]: https://github.com/bevyengine/bevy/pull/6047
[6039]: https://github.com/bevyengine/bevy/pull/6039
[6029]: https://github.com/bevyengine/bevy/pull/6029
[6025]: https://github.com/bevyengine/bevy/pull/6025
[6023]: https://github.com/bevyengine/bevy/pull/6023
[6020]: https://github.com/bevyengine/bevy/pull/6020
[6017]: https://github.com/bevyengine/bevy/pull/6017
[6015]: https://github.com/bevyengine/bevy/pull/6015
[6014]: https://github.com/bevyengine/bevy/pull/6014
[6009]: https://github.com/bevyengine/bevy/pull/6009
[6008]: https://github.com/bevyengine/bevy/pull/6008
[6000]: https://github.com/bevyengine/bevy/pull/6000
[5994]: https://github.com/bevyengine/bevy/pull/5994
[5993]: https://github.com/bevyengine/bevy/pull/5993
[5991]: https://github.com/bevyengine/bevy/pull/5991
[5988]: https://github.com/bevyengine/bevy/pull/5988
[5987]: https://github.com/bevyengine/bevy/pull/5987
[5982]: https://github.com/bevyengine/bevy/pull/5982
[5978]: https://github.com/bevyengine/bevy/pull/5978
[5971]: https://github.com/bevyengine/bevy/pull/5971
[5968]: https://github.com/bevyengine/bevy/pull/5968
[5957]: https://github.com/bevyengine/bevy/pull/5957
[5952]: https://github.com/bevyengine/bevy/pull/5952
[5949]: https://github.com/bevyengine/bevy/pull/5949
[5948]: https://github.com/bevyengine/bevy/pull/5948
[5947]: https://github.com/bevyengine/bevy/pull/5947
[5945]: https://github.com/bevyengine/bevy/pull/5945
[5942]: https://github.com/bevyengine/bevy/pull/5942
[5923]: https://github.com/bevyengine/bevy/pull/5923
[5917]: https://github.com/bevyengine/bevy/pull/5917
[5916]: https://github.com/bevyengine/bevy/pull/5916
[5895]: https://github.com/bevyengine/bevy/pull/5895
[5886]: https://github.com/bevyengine/bevy/pull/5886
[5883]: https://github.com/bevyengine/bevy/pull/5883
[5878]: https://github.com/bevyengine/bevy/pull/5878
[5877]: https://github.com/bevyengine/bevy/pull/5877
[5871]: https://github.com/bevyengine/bevy/pull/5871
[5865]: https://github.com/bevyengine/bevy/pull/5865
[5864]: https://github.com/bevyengine/bevy/pull/5864
[5863]: https://github.com/bevyengine/bevy/pull/5863
[5857]: https://github.com/bevyengine/bevy/pull/5857
[5855]: https://github.com/bevyengine/bevy/pull/5855
[5854]: https://github.com/bevyengine/bevy/pull/5854
[5853]: https://github.com/bevyengine/bevy/pull/5853
[5847]: https://github.com/bevyengine/bevy/pull/5847
[5838]: https://github.com/bevyengine/bevy/pull/5838
[5835]: https://github.com/bevyengine/bevy/pull/5835
[5827]: https://github.com/bevyengine/bevy/pull/5827
[5826]: https://github.com/bevyengine/bevy/pull/5826
[5825]: https://github.com/bevyengine/bevy/pull/5825
[5819]: https://github.com/bevyengine/bevy/pull/5819
[5814]: https://github.com/bevyengine/bevy/pull/5814
[5813]: https://github.com/bevyengine/bevy/pull/5813
[5806]: https://github.com/bevyengine/bevy/pull/5806
[5797]: https://github.com/bevyengine/bevy/pull/5797
[5783]: https://github.com/bevyengine/bevy/pull/5783
[5782]: https://github.com/bevyengine/bevy/pull/5782
[5780]: https://github.com/bevyengine/bevy/pull/5780
[5776]: https://github.com/bevyengine/bevy/pull/5776
[5766]: https://github.com/bevyengine/bevy/pull/5766
[5761]: https://github.com/bevyengine/bevy/pull/5761
[5758]: https://github.com/bevyengine/bevy/pull/5758
[5757]: https://github.com/bevyengine/bevy/pull/5757
[5752]: https://github.com/bevyengine/bevy/pull/5752
[5751]: https://github.com/bevyengine/bevy/pull/5751
[5747]: https://github.com/bevyengine/bevy/pull/5747
[5731]: https://github.com/bevyengine/bevy/pull/5731
[5730]: https://github.com/bevyengine/bevy/pull/5730
[5729]: https://github.com/bevyengine/bevy/pull/5729
[5728]: https://github.com/bevyengine/bevy/pull/5728
[5723]: https://github.com/bevyengine/bevy/pull/5723
[5720]: https://github.com/bevyengine/bevy/pull/5720
[5708]: https://github.com/bevyengine/bevy/pull/5708
[5688]: https://github.com/bevyengine/bevy/pull/5688
[5686]: https://github.com/bevyengine/bevy/pull/5686
[5685]: https://github.com/bevyengine/bevy/pull/5685
[5676]: https://github.com/bevyengine/bevy/pull/5676
[5666]: https://github.com/bevyengine/bevy/pull/5666
[5664]: https://github.com/bevyengine/bevy/pull/5664
[5658]: https://github.com/bevyengine/bevy/pull/5658
[5635]: https://github.com/bevyengine/bevy/pull/5635
[5630]: https://github.com/bevyengine/bevy/pull/5630
[5626]: https://github.com/bevyengine/bevy/pull/5626
[5613]: https://github.com/bevyengine/bevy/pull/5613
[5608]: https://github.com/bevyengine/bevy/pull/5608
[5601]: https://github.com/bevyengine/bevy/pull/5601
[5593]: https://github.com/bevyengine/bevy/pull/5593
[5587]: https://github.com/bevyengine/bevy/pull/5587
[5586]: https://github.com/bevyengine/bevy/pull/5586
[5582]: https://github.com/bevyengine/bevy/pull/5582
[5577]: https://github.com/bevyengine/bevy/pull/5577
[5566]: https://github.com/bevyengine/bevy/pull/5566
[5560]: https://github.com/bevyengine/bevy/pull/5560
[5558]: https://github.com/bevyengine/bevy/pull/5558
[5557]: https://github.com/bevyengine/bevy/pull/5557
[5556]: https://github.com/bevyengine/bevy/pull/5556
[5554]: https://github.com/bevyengine/bevy/pull/5554
[5546]: https://github.com/bevyengine/bevy/pull/5546
[5537]: https://github.com/bevyengine/bevy/pull/5537
[5531]: https://github.com/bevyengine/bevy/pull/5531
[5527]: https://github.com/bevyengine/bevy/pull/5527
[5512]: https://github.com/bevyengine/bevy/pull/5512
[5509]: https://github.com/bevyengine/bevy/pull/5509
[5495]: https://github.com/bevyengine/bevy/pull/5495
[5483]: https://github.com/bevyengine/bevy/pull/5483
[5481]: https://github.com/bevyengine/bevy/pull/5481
[5473]: https://github.com/bevyengine/bevy/pull/5473
[5461]: https://github.com/bevyengine/bevy/pull/5461
[5413]: https://github.com/bevyengine/bevy/pull/5413
[5409]: https://github.com/bevyengine/bevy/pull/5409
[5402]: https://github.com/bevyengine/bevy/pull/5402
[5340]: https://github.com/bevyengine/bevy/pull/5340
[5325]: https://github.com/bevyengine/bevy/pull/5325
[5321]: https://github.com/bevyengine/bevy/pull/5321
[5296]: https://github.com/bevyengine/bevy/pull/5296
[5264]: https://github.com/bevyengine/bevy/pull/5264
[5250]: https://github.com/bevyengine/bevy/pull/5250
[5205]: https://github.com/bevyengine/bevy/pull/5205
[5096]: https://github.com/bevyengine/bevy/pull/5096
[4992]: https://github.com/bevyengine/bevy/pull/4992
[4970]: https://github.com/bevyengine/bevy/pull/4970
[4919]: https://github.com/bevyengine/bevy/pull/4919
[4809]: https://github.com/bevyengine/bevy/pull/4809
[4800]: https://github.com/bevyengine/bevy/pull/4800
[4761]: https://github.com/bevyengine/bevy/pull/4761
[4724]: https://github.com/bevyengine/bevy/pull/4724
[4466]: https://github.com/bevyengine/bevy/pull/4466
[4460]: https://github.com/bevyengine/bevy/pull/4460
[4452]: https://github.com/bevyengine/bevy/pull/4452
[4352]: https://github.com/bevyengine/bevy/pull/4352
[4109]: https://github.com/bevyengine/bevy/pull/4109
[3425]: https://github.com/bevyengine/bevy/pull/3425
[3009]: https://github.com/bevyengine/bevy/pull/3009
[2975]: https://github.com/bevyengine/bevy/pull/2975

## Version 0.8.0 (2022-07-30)

### Added

- [Callable PBR functions][4939]
- [Spotlights][4715]
- [Camera Driven Rendering][4745]
- [Camera Driven Viewports][4898]
- [Visibilty Inheritance, universal `ComputedVisibility`, and `RenderLayers` support][5310]
- [Better Materials: `AsBindGroup` trait and derive, simpler `Material` trait][5053]
- [Derive `AsBindGroup` Improvements: Better errors, more options, update examples][5364]
- [Support `AsBindGroup` for 2d materials as well][5312]
- [Parallel Frustum Culling][4489]
- [Hierarchy commandization][4197]
- [Generate vertex tangents using mikktspace][3872]
- [Add a `SpatialBundle` with `Visibility` and `Transform` components][5344]
- [Add `RegularPolygon` and `Circle` meshes][3730]
- [Add a `SceneBundle` to spawn a scene][2424]
- [Allow higher order systems][4833]
- [Add global `init()` and `get()` accessors for all newtyped `TaskPools`][2250]
- [Add reusable shader functions for transforming position/normal/tangent][4901]
- [Add support for vertex colors][4528]
- [Add support for removing attributes from meshes][5254]
- [Add option to center a window][4999]
- [Add `depth_load_op` configuration field to `Camera3d`][4904]
- [Refactor `Camera` methods and add viewport rect][4948]
- [Add `TextureFormat::R16Unorm` support for `Image`][5249]
- [Add a `VisibilityBundle` with `Visibility` and `ComputedVisibility` components][5335]
- [Add ExtractResourcePlugin][3745]
- [Add depth_bias to SpecializedMaterial][4101]
- [Added `offset` parameter to `TextureAtlas::from_grid_with_padding`][4836]
- [Add the possibility to create custom 2d orthographic cameras][4048]
- [bevy_render: Add `attributes` and `attributes_mut` methods to `Mesh`][3927]
- [Add helper methods for rotating `Transform`s][5151]
- [Enable wgpu profiling spans when using bevy's trace feature][5182]
- [bevy_pbr: rework `extract_meshes`][4240]
- [Add `inverse_projection` and `inverse_view_proj` fields to shader view uniform][5119]
- [Add `ViewRangefinder3d` to reduce boilerplate when enqueuing standard 3D `PhaseItems`][5014]
- [Create `bevy_ptr` standalone crate][4653]
- [Add `IntoIterator` impls for `&Query` and `&mut Query`][4692]
- [Add untyped APIs for `Components` and `Resources`][4447]
- [Add infallible resource getters for `WorldCell`][4104]
- [Add `get_change_ticks` method to `EntityRef` and `EntityMut`][2539]
- [Add comparison methods to `FilteredAccessSet`][4211]
- [Add `Commands::new_from_entities`][4423]
- [Add `QueryState::get_single_unchecked_manual` and its family members][4841]
- [Add `ParallelCommands` system parameter][4749]
- [Add methods for querying lists of entities][4879]
- [Implement `FusedIterator` for eligible `Iterator` types][4942]
- [Add `component_id()` function to `World` and `Components`][5066]
- [Add ability to inspect entity's components][5136]
- [Add a more helpful error to help debug panicking command on despawned entity][5198]
- [Add `ExactSizeIterator` implementation for `QueryCombinatonIter`][5148]
- [Added the `ignore_fields` attribute to the derive macros for `*Label` types][5366]
- [Exact sized event iterators][3863]
- [Add a `clear()` method to the `EventReader` that consumes the iterator][4693]
- [Add helpers to send `Events` from `World`][5355]
- [Add file metadata to `AssetIo`][2123]
- [Add missing audio/ogg file extensions: .oga, .spx][4703]
- [Add `reload_asset` method to `AssetServer`][5106]
- [Allow specifying chrome tracing file path using an environment variable][4618]
- [Create a simple tool to compare traces between executions][4628]
- [Add a tracing span for run criteria][4709]
- [Add tracing spans for `Query::par_for_each` and its variants.][4711]
- [Add a `release_all` method on `Input`][5011]
- [Add a `reset_all` method on `Input`][5015]
- [Add a helper tool to build examples for wasm][4776]
- [bevy_reflect: add a `ReflectFromPtr` type to create `&dyn Reflect` from a `*const ()`][4475]
- [Add a `ReflectDefault` type and add `#[reflect(Default)]` to all component types that implement Default and are user facing][3733]
- [Add a macro to implement `Reflect` for struct types and migrate glam types to use this for reflection][4540]
- [bevy_reflect: reflect arrays][4701]
- [bevy_reflect: reflect char][4790]
- [bevy_reflect: add `GetTypeRegistration` impl for reflected tuples][4226]
- [Add reflection for `Resources`][5175]
- [bevy_reflect: add `as_reflect` and `as_reflect_mut` methods on `Reflect`][4350]
- [Add an `apply_or_insert` method to `ReflectResource` and `ReflectComponent`][5201]
- [bevy_reflect: `IntoIter` for `DynamicList` and `DynamicMap`][4108]
- [bevy_reflect: Add `PartialEq` to reflected `f32`s and `f64`s][4217]
- [Create mutable versions of `TypeRegistry` methods][4484]
- [bevy_reflect: add a `get_boxed` method to `reflect_trait`][4120]
- [bevy_reflect: add `#[reflect(default)]` attribute for `FromReflect`][4140]
- [bevy_reflect: add statically available type info for reflected types][4042]
- [Add an `assert_is_exclusive_system` function][5275]
- [bevy_ui: add a multi-windows check for `Interaction` (we dont yet support multiple windows)][5225]

### Changed

- [Depend on Taffy (a Dioxus and Bevy-maintained fork of Stretch)][4716]
- [Use lifetimed, type erased pointers in bevy_ecs][3001]
- [Migrate to `encase` from `crevice`][4339]
- [Update `wgpu` to 0.13][5168]
- [Pointerfication followup: Type safety and cleanup][4621]
- [bevy_ptr works in no_std environments][4760]
- [Fail to compile on 16-bit platforms][4736]
- [Improve ergonomics and reduce boilerplate around creating text elements][5343]
- [Don't cull `Ui` nodes that have a rotation][5389]
- [Rename `ElementState` to `ButtonState`][4314]
- [Move `Size` to `bevy_ui`][4285]
- [Move `Rect` to `bevy_ui` and rename it to `UiRect`][4276]
- [Modify `FontAtlas` so that it can handle fonts of any size][3592]
- [Rename `CameraUi`][5234]
- [Remove `task_pool` parameter from `par_for_each(_mut)`][4705]
- [Copy `TaskPool` resoures to sub-Apps][4792]
- [Allow closing windows at runtime][3575]
- [Move the configuration of the `WindowPlugin` to a `Resource`][5227]
- [Optionally resize `Window` canvas element to fit parent element][4726]
- [Change window resolution types from tuple to `Vec2`][5276]
- [Update time by sending frame `Instant` through a channel][4744]
- [Split time functionality into `bevy_time`][4187]
- [Split mesh shader files to make the shaders more reusable][4867]
- [Set `naga` capabilities corresponding to `wgpu` features][4824]
- [Separate out PBR lighting, shadows, clustered forward, and utils from pbr.wgsl][4938]
- [Separate PBR and tonemapping into 2 functions][5078]
- [Make `RenderStage::Extract` run on the render world][4402]
- [Change default `FilterMode` of `Image` to `Linear`][4465]
- [bevy_render: Fix KTX2 UASTC format mapping][4569]
- [Allow rendering meshes without UV coordinate data][5222]
- [Validate vertex attribute format on insertion][5259]
- [Use `Affine3A` for `GlobalTransform`to allow any affine transformation][4379]
- [Recalculate entity `AABB`s when meshes change][4944]
- [Change `check_visibility` to use thread-local queues instead of a channel][4663]
- [Allow unbatched render phases to use unstable sorts][5049]
- [Extract resources into their target location][5271]
- [Enable loading textures of unlimited size][5305]
- [Do not create nor execute render passes which have no `PhaseItems` to draw][4643]
- [Filter material handles on extraction][4178]
- [Apply vertex colors to `ColorMaterial` and `Mesh2D`][4812]
- [Make `MaterialPipelineKey` fields public][4508]
- [Simplified API to get NDC from camera and world position][4041]
- [Set `alpha_mode` based on alpha value][4658]
- [Make `Wireframe` respect `VisibleEntities`][4660]
- [Use const `Vec2` in lights cluster and bounding box when possible][4602]
- [Make accessors for mesh vertices and indices public][3906]
- [Use `BufferUsages::UNIFORM` for `SkinnedMeshUniform`][4816]
- [Place origin of `OrthographicProjection` at integer pixel when using `ScalingMode::WindowSize`][4085]
- [Make `ScalingMode` more flexible][3253]
- [Move texture sample out of branch in `prepare_normal`][5129]
- [Make the fields of the `Material2dKey` public][5212]
- [Use collect to build mesh attributes][5255]
- [Replace `ReadOnlyFetch` with `ReadOnlyWorldQuery`][4626]
- [Replace `ComponentSparseSet`'s internals with a `Column`][4909]
- [Remove QF generics from all `Query/State` methods and types][5170]
- [Remove `.system()`][4499]
- [Make change lifespan deterministic and update docs][3956]
- [Make derived `SystemParam` readonly if possible][4650]
- [Merge `matches_archetype` and `matches_table`][4807]
- [Allows conversion of mutable queries to immutable queries][5376]
- [Skip `drop` when `needs_drop` is `false`][4773]
- [Use u32 over usize for `ComponentSparseSet` indicies][4723]
- [Remove redundant `ComponentId` in `Column`][4855]
- [Directly copy moved `Table` components to the target location][5056]
- [`SystemSet::before` and `SystemSet::after` now take `AsSystemLabel`][4503]
- [Converted exclusive systems to parallel systems wherever possible][2774]
- [Improve `size_hint` on `QueryIter`][4244]
- [Improve debugging tools for change detection][4160]
- [Make `RunOnce` a non-manual `System` impl][3922]
- [Apply buffers in `ParamSet`][4677]
- [Don't allocate for `ComponentDescriptors` of non-dynamic component types][4725]
- [Mark mutable APIs under ECS storage as `pub(crate)`][5065]
- [Update `ExactSizeIterator` impl to support archetypal filters (`With`, `Without`)][5124]
- [Removed world cell from places where split multable access is not needed][5167]
- [Add Events to `bevy_ecs` prelude][5159]
- [Improve `EntityMap` API][5231]
- [Implement `From<bool>` for `ShouldRun`.][5306]
- [Allow iter combinations on custom world queries][5286]
- [Simplify design for `*Label`s][4957]
- [Tidy up the code of events][4713]
- [Rename `send_default_event` to `send_event_default` on world][5383]
- [enable optional dependencies to stay optional][5023]
- [Remove the dependency cycles][5171]
- [Enforce type safe usage of Handle::get][4794]
- [Export anyhow::error for custom asset loaders][5359]
- [Update `shader_material_glsl` example to include texture sampling][5215]
- [Remove unused code in game of life shader][5349]
- [Make the contributor birbs bounce to the window height][5274]
- [Improve Gamepad D-Pad Button Detection][5220]
- [bevy_reflect: support map insertio][5173]
- [bevy_reflect: improve debug formatting for reflected types][4218]
- [bevy_reflect_derive: big refactor tidying up the code][4712]
- [bevy_reflect: small refactor and default `Reflect` methods][4739]
- [Make `Reflect` safe to implement][5010]
- [`bevy_reflect`: put `serialize` into external `ReflectSerialize` type][4782]
- [Remove `Serialize` impl for `dyn Array` and friends][4780]
- [Re-enable `#[derive(TypeUuid)]` for generics][4118]
- [Move primitive type registration into `bevy_reflect`][4844]
- [Implement reflection for more `glam` types][5194]
- [Make `reflect_partial_eq` return more accurate results][5210]
- [Make public macros more robust with `$crate`][4655]
- [Ensure that the parent is always the expected entity][4717]
- [Support returning data out of `with_children`][4708]
- [Remove `EntityMut::get_unchecked`][4547]
- [Diagnostics: meaningful error when graph node has wrong number of inputs][4924]
- [Remove redundant `Size` import][5339]
- [Export and register `Mat2`.][5324]
- [Implement `Debug` for `Gamepads`][5291]
- [Update codebase to use `IntoIterator` where possible.][5269]
- [Rename `headless_defaults` example to `no_renderer` for clarity][5263]
- [Remove dead `SystemLabelMarker` struct][5190]
- [bevy_reflect: remove `glam` from a test which is active without the glam feature][5195]
- [Disable vsync for stress tests][5187]
- [Move `get_short_name` utility method from `bevy_reflect` into `bevy_utils`][5174]
- [Derive `Default` for enums where possible][5158]
- [Implement `Eq` and `PartialEq` for `MouseScrollUnit`][5048]
- [Some cleanup for `bevy_ptr`][4668]
- [Move float_ord from `bevy_core` to `bevy_utils`][4189]
- [Remove unused `CountdownEvent`][4290]
- [Some minor cleanups of asset_server][4604]
- [Use `elapsed()` on `Instant`][4599]
- [Make paused `Timers` update `just_finished` on tick][4445]
- [bevy_utils: remove hardcoded log level limit][4580]
- [Make `Time::update_with_instant` public for use in tests][4469]
- [Do not impl Component for Task][4113]
- [Remove nonexistent `WgpuResourceDiagnosticsPlugin`][4541]
- [Update ndk-glue requirement from 0.5 to 0.6][3624]
- [Update tracing-tracy requirement from 0.8.0 to 0.9.0][4786]
- [update image to 0.24][4121]
- [update xshell to 0.2][4789]
- [Update gilrs to v0.9][4848]
- [bevy_log: upgrade to tracing-tracy 0.10.0][4991]
- [update hashbrown to 0.12][5035]
- [Update `clap` to 3.2 in tools using `value_parser`][5031]
- [Updated `glam` to `0.21`.][5142]
- [Update Notify Dependency][5396]

### Fixed

- [bevy_ui: keep `Color` as 4 `f32`s][4494]
- [Fix issues with bevy on android other than the rendering][5130]
- [Update layout/style when scale factor changes too][4689]
- [Fix `Overflow::Hidden` so it works correctly with `scale_factor_override`][3854]
- [Fix `bevy_ui` touch input][4099]
- [Fix physical viewport calculation][5055]
- [Minimally fix the known unsoundness in `bevy_mikktspace`][5299]
- [Make `Transform` propagation correct in the presence of updated children][4608]
- [`StorageBuffer` uses wrong type to calculate the buffer size.][4557]
- [Fix confusing near and far fields in Camera][4457]
- [Allow minimising window if using a 2d camera][4527]
- [WGSL: use correct syntax for matrix access][5039]
- [Gltf: do not import `IoTaskPool` in wasm][5038]
- [Fix skinned mesh normal handling in mesh shader][5095]
- [Don't panic when `StandardMaterial` `normal_map` hasn't loaded yet][5307]
- [Fix incorrect rotation in `Transform::rotate_around`][5300]
- [Fix `extract_wireframes`][5301]
- [Fix type parameter name conflicts of `#[derive(Bundle)]`][4636]
- [Remove unnecessary `unsafe impl` of `Send+Sync` for `ParallelSystemContainer`][5137]
- [Fix line material shader][5348]
- [Fix `mouse_clicked` check for touch][2029]
- [Fix unsoundness with `Or`/`AnyOf`/`Option` component access][4659]
- [Improve soundness of `CommandQueue`][4863]
- [Fix some memory leaks detected by miri][4959]
- [Fix Android example icon][4076]
- [Fix broken `WorldCell` test][5009]
- [Bugfix `State::set` transition condition infinite loop][4890]
- [Fix crash when using `Duration::MAX`][4900]
- [Fix release builds: Move asserts under `#[cfg(debug_assertions)]`][4871]
- [Fix frame count being a float][4493]
- [Fix "unused" warnings when compiling with `render` feature but without `animation`][4714]
- [Fix re-adding a plugin to a `PluginGroup`][2039]
- [Fix torus normals][4520]
- [Add `NO_STORAGE_BUFFERS_SUPPORT` shaderdef when needed][4949]

[2029]: https://github.com/bevyengine/bevy/pull/2029
[2039]: https://github.com/bevyengine/bevy/pull/2039
[2123]: https://github.com/bevyengine/bevy/pull/2123
[2250]: https://github.com/bevyengine/bevy/pull/2250
[2424]: https://github.com/bevyengine/bevy/pull/2424
[2539]: https://github.com/bevyengine/bevy/pull/2539
[2774]: https://github.com/bevyengine/bevy/pull/2774
[3001]: https://github.com/bevyengine/bevy/pull/3001
[3253]: https://github.com/bevyengine/bevy/pull/3253
[3575]: https://github.com/bevyengine/bevy/pull/3575
[3592]: https://github.com/bevyengine/bevy/pull/3592
[3624]: https://github.com/bevyengine/bevy/pull/3624
[3730]: https://github.com/bevyengine/bevy/pull/3730
[3733]: https://github.com/bevyengine/bevy/pull/3733
[3745]: https://github.com/bevyengine/bevy/pull/3745
[3854]: https://github.com/bevyengine/bevy/pull/3854
[3863]: https://github.com/bevyengine/bevy/pull/3863
[3872]: https://github.com/bevyengine/bevy/pull/3872
[3906]: https://github.com/bevyengine/bevy/pull/3906
[3922]: https://github.com/bevyengine/bevy/pull/3922
[3927]: https://github.com/bevyengine/bevy/pull/3927
[3956]: https://github.com/bevyengine/bevy/pull/3956
[4041]: https://github.com/bevyengine/bevy/pull/4041
[4042]: https://github.com/bevyengine/bevy/pull/4042
[4048]: https://github.com/bevyengine/bevy/pull/4048
[4076]: https://github.com/bevyengine/bevy/pull/4076
[4085]: https://github.com/bevyengine/bevy/pull/4085
[4099]: https://github.com/bevyengine/bevy/pull/4099
[4101]: https://github.com/bevyengine/bevy/pull/4101
[4104]: https://github.com/bevyengine/bevy/pull/4104
[4108]: https://github.com/bevyengine/bevy/pull/4108
[4113]: https://github.com/bevyengine/bevy/pull/4113
[4118]: https://github.com/bevyengine/bevy/pull/4118
[4120]: https://github.com/bevyengine/bevy/pull/4120
[4121]: https://github.com/bevyengine/bevy/pull/4121
[4140]: https://github.com/bevyengine/bevy/pull/4140
[4160]: https://github.com/bevyengine/bevy/pull/4160
[4178]: https://github.com/bevyengine/bevy/pull/4178
[4187]: https://github.com/bevyengine/bevy/pull/4187
[4189]: https://github.com/bevyengine/bevy/pull/4189
[4197]: https://github.com/bevyengine/bevy/pull/4197
[4211]: https://github.com/bevyengine/bevy/pull/4211
[4217]: https://github.com/bevyengine/bevy/pull/4217
[4218]: https://github.com/bevyengine/bevy/pull/4218
[4226]: https://github.com/bevyengine/bevy/pull/4226
[4240]: https://github.com/bevyengine/bevy/pull/4240
[4244]: https://github.com/bevyengine/bevy/pull/4244
[4276]: https://github.com/bevyengine/bevy/pull/4276
[4285]: https://github.com/bevyengine/bevy/pull/4285
[4290]: https://github.com/bevyengine/bevy/pull/4290
[4314]: https://github.com/bevyengine/bevy/pull/4314
[4339]: https://github.com/bevyengine/bevy/pull/4339
[4350]: https://github.com/bevyengine/bevy/pull/4350
[4379]: https://github.com/bevyengine/bevy/pull/4379
[4402]: https://github.com/bevyengine/bevy/pull/4402
[4423]: https://github.com/bevyengine/bevy/pull/4423
[4445]: https://github.com/bevyengine/bevy/pull/4445
[4447]: https://github.com/bevyengine/bevy/pull/4447
[4457]: https://github.com/bevyengine/bevy/pull/4457
[4465]: https://github.com/bevyengine/bevy/pull/4465
[4469]: https://github.com/bevyengine/bevy/pull/4469
[4475]: https://github.com/bevyengine/bevy/pull/4475
[4484]: https://github.com/bevyengine/bevy/pull/4484
[4489]: https://github.com/bevyengine/bevy/pull/4489
[4493]: https://github.com/bevyengine/bevy/pull/4493
[4494]: https://github.com/bevyengine/bevy/pull/4494
[4499]: https://github.com/bevyengine/bevy/pull/4499
[4503]: https://github.com/bevyengine/bevy/pull/4503
[4508]: https://github.com/bevyengine/bevy/pull/4508
[4520]: https://github.com/bevyengine/bevy/pull/4520
[4527]: https://github.com/bevyengine/bevy/pull/4527
[4528]: https://github.com/bevyengine/bevy/pull/4528
[4540]: https://github.com/bevyengine/bevy/pull/4540
[4541]: https://github.com/bevyengine/bevy/pull/4541
[4547]: https://github.com/bevyengine/bevy/pull/4547
[4557]: https://github.com/bevyengine/bevy/pull/4557
[4569]: https://github.com/bevyengine/bevy/pull/4569
[4580]: https://github.com/bevyengine/bevy/pull/4580
[4599]: https://github.com/bevyengine/bevy/pull/4599
[4602]: https://github.com/bevyengine/bevy/pull/4602
[4604]: https://github.com/bevyengine/bevy/pull/4604
[4608]: https://github.com/bevyengine/bevy/pull/4608
[4618]: https://github.com/bevyengine/bevy/pull/4618
[4621]: https://github.com/bevyengine/bevy/pull/4621
[4626]: https://github.com/bevyengine/bevy/pull/4626
[4628]: https://github.com/bevyengine/bevy/pull/4628
[4636]: https://github.com/bevyengine/bevy/pull/4636
[4643]: https://github.com/bevyengine/bevy/pull/4643
[4650]: https://github.com/bevyengine/bevy/pull/4650
[4653]: https://github.com/bevyengine/bevy/pull/4653
[4655]: https://github.com/bevyengine/bevy/pull/4655
[4658]: https://github.com/bevyengine/bevy/pull/4658
[4659]: https://github.com/bevyengine/bevy/pull/4659
[4660]: https://github.com/bevyengine/bevy/pull/4660
[4663]: https://github.com/bevyengine/bevy/pull/4663
[4668]: https://github.com/bevyengine/bevy/pull/4668
[4677]: https://github.com/bevyengine/bevy/pull/4677
[4689]: https://github.com/bevyengine/bevy/pull/4689
[4692]: https://github.com/bevyengine/bevy/pull/4692
[4693]: https://github.com/bevyengine/bevy/pull/4693
[4701]: https://github.com/bevyengine/bevy/pull/4701
[4703]: https://github.com/bevyengine/bevy/pull/4703
[4705]: https://github.com/bevyengine/bevy/pull/4705
[4708]: https://github.com/bevyengine/bevy/pull/4708
[4709]: https://github.com/bevyengine/bevy/pull/4709
[4711]: https://github.com/bevyengine/bevy/pull/4711
[4712]: https://github.com/bevyengine/bevy/pull/4712
[4713]: https://github.com/bevyengine/bevy/pull/4713
[4714]: https://github.com/bevyengine/bevy/pull/4714
[4715]: https://github.com/bevyengine/bevy/pull/4715
[4716]: https://github.com/bevyengine/bevy/pull/4716
[4717]: https://github.com/bevyengine/bevy/pull/4717
[4723]: https://github.com/bevyengine/bevy/pull/4723
[4725]: https://github.com/bevyengine/bevy/pull/4725
[4726]: https://github.com/bevyengine/bevy/pull/4726
[4736]: https://github.com/bevyengine/bevy/pull/4736
[4739]: https://github.com/bevyengine/bevy/pull/4739
[4744]: https://github.com/bevyengine/bevy/pull/4744
[4745]: https://github.com/bevyengine/bevy/pull/4745
[4749]: https://github.com/bevyengine/bevy/pull/4749
[4760]: https://github.com/bevyengine/bevy/pull/4760
[4773]: https://github.com/bevyengine/bevy/pull/4773
[4776]: https://github.com/bevyengine/bevy/pull/4776
[4780]: https://github.com/bevyengine/bevy/pull/4780
[4782]: https://github.com/bevyengine/bevy/pull/4782
[4786]: https://github.com/bevyengine/bevy/pull/4786
[4789]: https://github.com/bevyengine/bevy/pull/4789
[4790]: https://github.com/bevyengine/bevy/pull/4790
[4792]: https://github.com/bevyengine/bevy/pull/4792
[4794]: https://github.com/bevyengine/bevy/pull/4794
[4807]: https://github.com/bevyengine/bevy/pull/4807
[4812]: https://github.com/bevyengine/bevy/pull/4812
[4816]: https://github.com/bevyengine/bevy/pull/4816
[4824]: https://github.com/bevyengine/bevy/pull/4824
[4833]: https://github.com/bevyengine/bevy/pull/4833
[4836]: https://github.com/bevyengine/bevy/pull/4836
[4841]: https://github.com/bevyengine/bevy/pull/4841
[4844]: https://github.com/bevyengine/bevy/pull/4844
[4848]: https://github.com/bevyengine/bevy/pull/4848
[4855]: https://github.com/bevyengine/bevy/pull/4855
[4863]: https://github.com/bevyengine/bevy/pull/4863
[4867]: https://github.com/bevyengine/bevy/pull/4867
[4871]: https://github.com/bevyengine/bevy/pull/4871
[4879]: https://github.com/bevyengine/bevy/pull/4879
[4890]: https://github.com/bevyengine/bevy/pull/4890
[4898]: https://github.com/bevyengine/bevy/pull/4898
[4900]: https://github.com/bevyengine/bevy/pull/4900
[4901]: https://github.com/bevyengine/bevy/pull/4901
[4904]: https://github.com/bevyengine/bevy/pull/4904
[4909]: https://github.com/bevyengine/bevy/pull/4909
[4924]: https://github.com/bevyengine/bevy/pull/4924
[4938]: https://github.com/bevyengine/bevy/pull/4938
[4939]: https://github.com/bevyengine/bevy/pull/4939
[4942]: https://github.com/bevyengine/bevy/pull/4942
[4944]: https://github.com/bevyengine/bevy/pull/4944
[4948]: https://github.com/bevyengine/bevy/pull/4948
[4949]: https://github.com/bevyengine/bevy/pull/4949
[4957]: https://github.com/bevyengine/bevy/pull/4957
[4959]: https://github.com/bevyengine/bevy/pull/4959
[4991]: https://github.com/bevyengine/bevy/pull/4991
[4999]: https://github.com/bevyengine/bevy/pull/4999
[5009]: https://github.com/bevyengine/bevy/pull/5009
[5010]: https://github.com/bevyengine/bevy/pull/5010
[5011]: https://github.com/bevyengine/bevy/pull/5011
[5014]: https://github.com/bevyengine/bevy/pull/5014
[5015]: https://github.com/bevyengine/bevy/pull/5015
[5023]: https://github.com/bevyengine/bevy/pull/5023
[5031]: https://github.com/bevyengine/bevy/pull/5031
[5035]: https://github.com/bevyengine/bevy/pull/5035
[5038]: https://github.com/bevyengine/bevy/pull/5038
[5039]: https://github.com/bevyengine/bevy/pull/5039
[5048]: https://github.com/bevyengine/bevy/pull/5048
[5049]: https://github.com/bevyengine/bevy/pull/5049
[5053]: https://github.com/bevyengine/bevy/pull/5053
[5055]: https://github.com/bevyengine/bevy/pull/5055
[5056]: https://github.com/bevyengine/bevy/pull/5056
[5065]: https://github.com/bevyengine/bevy/pull/5065
[5066]: https://github.com/bevyengine/bevy/pull/5066
[5078]: https://github.com/bevyengine/bevy/pull/5078
[5095]: https://github.com/bevyengine/bevy/pull/5095
[5106]: https://github.com/bevyengine/bevy/pull/5106
[5119]: https://github.com/bevyengine/bevy/pull/5119
[5124]: https://github.com/bevyengine/bevy/pull/5124
[5129]: https://github.com/bevyengine/bevy/pull/5129
[5130]: https://github.com/bevyengine/bevy/pull/5130
[5136]: https://github.com/bevyengine/bevy/pull/5136
[5137]: https://github.com/bevyengine/bevy/pull/5137
[5142]: https://github.com/bevyengine/bevy/pull/5142
[5148]: https://github.com/bevyengine/bevy/pull/5148
[5151]: https://github.com/bevyengine/bevy/pull/5151
[5158]: https://github.com/bevyengine/bevy/pull/5158
[5159]: https://github.com/bevyengine/bevy/pull/5159
[5167]: https://github.com/bevyengine/bevy/pull/5167
[5168]: https://github.com/bevyengine/bevy/pull/5168
[5170]: https://github.com/bevyengine/bevy/pull/5170
[5171]: https://github.com/bevyengine/bevy/pull/5171
[5173]: https://github.com/bevyengine/bevy/pull/5173
[5174]: https://github.com/bevyengine/bevy/pull/5174
[5175]: https://github.com/bevyengine/bevy/pull/5175
[5182]: https://github.com/bevyengine/bevy/pull/5182
[5187]: https://github.com/bevyengine/bevy/pull/5187
[5190]: https://github.com/bevyengine/bevy/pull/5190
[5194]: https://github.com/bevyengine/bevy/pull/5194
[5195]: https://github.com/bevyengine/bevy/pull/5195
[5198]: https://github.com/bevyengine/bevy/pull/5198
[5201]: https://github.com/bevyengine/bevy/pull/5201
[5210]: https://github.com/bevyengine/bevy/pull/5210
[5212]: https://github.com/bevyengine/bevy/pull/5212
[5215]: https://github.com/bevyengine/bevy/pull/5215
[5220]: https://github.com/bevyengine/bevy/pull/5220
[5222]: https://github.com/bevyengine/bevy/pull/5222
[5225]: https://github.com/bevyengine/bevy/pull/5225
[5227]: https://github.com/bevyengine/bevy/pull/5227
[5231]: https://github.com/bevyengine/bevy/pull/5231
[5234]: https://github.com/bevyengine/bevy/pull/5234
[5249]: https://github.com/bevyengine/bevy/pull/5249
[5254]: https://github.com/bevyengine/bevy/pull/5254
[5255]: https://github.com/bevyengine/bevy/pull/5255
[5259]: https://github.com/bevyengine/bevy/pull/5259
[5263]: https://github.com/bevyengine/bevy/pull/5263
[5269]: https://github.com/bevyengine/bevy/pull/5269
[5271]: https://github.com/bevyengine/bevy/pull/5271
[5274]: https://github.com/bevyengine/bevy/pull/5274
[5275]: https://github.com/bevyengine/bevy/pull/5275
[5276]: https://github.com/bevyengine/bevy/pull/5276
[5286]: https://github.com/bevyengine/bevy/pull/5286
[5291]: https://github.com/bevyengine/bevy/pull/5291
[5299]: https://github.com/bevyengine/bevy/pull/5299
[5300]: https://github.com/bevyengine/bevy/pull/5300
[5301]: https://github.com/bevyengine/bevy/pull/5301
[5305]: https://github.com/bevyengine/bevy/pull/5305
[5306]: https://github.com/bevyengine/bevy/pull/5306
[5307]: https://github.com/bevyengine/bevy/pull/5307
[5310]: https://github.com/bevyengine/bevy/pull/5310
[5312]: https://github.com/bevyengine/bevy/pull/5312
[5324]: https://github.com/bevyengine/bevy/pull/5324
[5335]: https://github.com/bevyengine/bevy/pull/5335
[5339]: https://github.com/bevyengine/bevy/pull/5339
[5343]: https://github.com/bevyengine/bevy/pull/5343
[5344]: https://github.com/bevyengine/bevy/pull/5344
[5348]: https://github.com/bevyengine/bevy/pull/5348
[5349]: https://github.com/bevyengine/bevy/pull/5349
[5355]: https://github.com/bevyengine/bevy/pull/5355
[5359]: https://github.com/bevyengine/bevy/pull/5359
[5364]: https://github.com/bevyengine/bevy/pull/5364
[5366]: https://github.com/bevyengine/bevy/pull/5366
[5376]: https://github.com/bevyengine/bevy/pull/5376
[5383]: https://github.com/bevyengine/bevy/pull/5383
[5389]: https://github.com/bevyengine/bevy/pull/5389
[5396]: https://github.com/bevyengine/bevy/pull/5396

## Version 0.7.0 (2022-04-15)

### Added

- [Mesh Skinning][4238]
- [Animation Player][4375]
- [Gltf animations][3751]
- [Mesh vertex buffer layouts][3959]
- [Render to a texture][3412]
- [KTX2/DDS/.basis compressed texture support][3884]
- [Audio control - play, pause, volume, speed, loop][3948]
- [Auto-label function systems with SystemTypeIdLabel][4224]
- [Query::get_many][4298]
- [Dynamic light clusters][3968]
- [Always update clusters and remove per-frame allocations][4169]
- [`ParamSet` for conflicting `SystemParam`:s][2765]
- [default() shorthand][4071]
- [use marker components for cameras instead of name strings][3635]
- [Implement `WorldQuery` derive macro][2713]
- [Implement AnyOf queries][2889]
- [Compute Pipeline Specialization][3979]
- [Make get_resource (and friends) infallible][4047]
- [bevy_pbr: Support flipping tangent space normal map y for DirectX normal maps][4433]
- [Faster view frustum culling][4181]
- [Use storage buffers for clustered forward point lights][3989]
- [Add &World as SystemParam][2923]
- [Add text wrapping support to Text2d][4347]
- [Scene Viewer to display glTF files][4183]
- [Internal Asset Hot Reloading][3966]
- [Add FocusPolicy to NodeBundle and ImageBundle][3952]
- [Allow iter combinations on queries with filters][3656]
- [bevy_render: Support overriding wgpu features and limits][3912]
- [bevy_render: Use RenderDevice to get limits/features and expose AdapterInfo][3931]
- [Reduce power usage with configurable event loop][3974]
- [can specify an anchor for a sprite][3463]
- [Implement len and is_empty for EventReaders][2969]
- [Add more FromWorld implementations][3945]
- [Add cart's fork of ecs_bench_suite][4225]
- [bevy_derive: Add derives for `Deref` and `DerefMut`][4328]
- [Add clear_schedule][3941]
- [Add Query::contains][3090]
- [bevy_render: Support removal of nodes, edges, subgraphs][3048]
- [Implement init_resource for `Commands` and `World`][3079]
- [Added method to restart the current state][3328]
- [Simplify sending empty events][2935]
- [impl Command for `impl FnOnce(&mut World)`][2996]
- [Useful error message when two assets have the save UUID][3739]
- [bevy_asset: Add AssetServerSettings watch_for_changes member][3643]
- [Add conversio from Color to u32][4088]
- [Introduce `SystemLabel`'s for `RenderAssetPlugin`, and change `Image` preparation system to run before others][3917]
- [Add a helper for storage buffers similar to `UniformVec`][4079]
- [StandardMaterial: expose a cull_mode option][3982]
- [Expose draw indirect][4056]
- [Add view transform to view uniform][3885]
- [Add a size method on Image.][3696]
- [add Visibility for lights][3958]
- [bevy_render: Provide a way to opt-out of the built-in frustum culling][3711]
- [use error scope to handle errors on shader module creation][3675]
- [include sources in shader validation error][3724]
- [insert the gltf mesh name on the entity if there is one][4119]
- [expose extras from gltf nodes][2154]
- [gltf: add a name to nodes without names][4396]
- [Enable drag-and-drop events on windows][3772]
- [Add transform hierarchy stress test][4170]
- [Add TransformBundle][3054]
- [Add Transform::rotate_around method][3107]
- [example on how to create an animation in code][4399]
- [Add examples for Transforms][2441]
- [Add mouse grab example][4114]
- [examples: add screenspace texture shader example][4063]
- [Add generic systems example][2636]
- [add examples on how to have a data source running in another thread / in a task pool thread][2915]
- [Simple 2d rotation example][3065]
- [Add move sprite example.][2414]
- [add an example using UI & states to create a game menu][2960]
- [CI runs `cargo miri test -p bevy_ecs`][4310]
- [Tracy spans around main 3D passes][4182]
- [Add automatic docs deployment to GitHub Pages][3535]

### Changed

- [Proper prehashing][3963]
- [Move import_path definitions into shader source][3976]
- [Make `System` responsible for updating its own archetypes][4115]
- [Some small changes related to run criteria piping][3923]
- [Remove unnecessary system labels][4340]
- [Increment last event count on next instead of iter][2382]
- [Obviate the need for `RunSystem`, and remove it][3817]
- [Cleanup some things which shouldn't be components][2982]
- [Remove the config api][3633]
- [Deprecate `.system`][3302]
- [Hide docs for concrete impls of Fetch, FetchState, and SystemParamState][4250]
- [Move the CoreStage::Startup to a seperate StartupSchedule label][2434]
- [`iter_mut` on Assets: send modified event only when asset is iterated over][3565]
- [check if resource for asset already exists before adding it][3560]
- [bevy_render: Batch insertion for prepare_uniform_components][4179]
- [Change default `ColorMaterial` color to white][3981]
- [bevy_render: Only auto-disable mappable primary buffers for discrete GPUs][3803]
- [bevy_render: Do not automatically enable MAPPABLE_PRIMARY_BUFFERS][3698]
- [increase the maximum number of point lights with shadows to the max supported by the device][4435]
- [perf: only recalculate frusta of changed lights][4086]
- [bevy_pbr: Optimize assign_lights_to_clusters][3984]
- [improve error messages for render graph runner][3930]
- [Skinned extraction speedup][4428]
- [Sprites - keep color as 4 f32][4361]
- [Change scaling mode to FixedHorizontal][4055]
- [Replace VSync with PresentMode][3812]
- [do not set cursor grab on window creation if not asked for][3617]
- [bevy_transform: Use Changed in the query for much faster transform_propagate_system][4180]
- [Split bevy_hierarchy out from bevy_transform][4168]
- [Make transform builder methods const][3045]
- [many_cubes: Add a cube pattern suitable for benchmarking culling changes][4126]
- [Make many_cubes example more interesting][4015]
- [Run tests (including doc tests) in `cargo run -p ci` command][3849]
- [Use more ergonomic span syntax][4246]

### Fixed

- [Remove unsound lifetime annotations on `EntityMut`][4096]
- [Remove unsound lifetime annotations on `Query` methods][4243]
- [Remove `World::components_mut`][4092]
- [unsafeify `World::entities_mut`][4093]
- [Use ManuallyDrop instead of forget in insert_resource_with_id][2947]
- [Backport soundness fix][3685]
- [Fix clicked UI nodes getting reset when hovering child nodes][4194]
- [Fix ui interactions when cursor disappears suddenly][3926]
- [Fix node update][3785]
- [Fix derive(SystemParam) macro][4400]
- [SystemParam Derive fixes][2838]
- [Do not crash if RenderDevice doesn't exist][4427]
- [Fixed case of R == G, following original conversion formula][4383]
- [Fixed the frustum-sphere collision and added tests][4035]
- [bevy_render: Fix Quad flip][3741]
- [Fix HDR asset support][3795]
- [fix cluster tiling calculations][4148]
- [bevy_pbr: Do not panic when more than 256 point lights are added the scene][3697]
- [fix issues with too many point lights][3916]
- [shader preprocessor - do not import if scope is not valid][4012]
- [support all line endings in shader preprocessor][3603]
- [Fix animation: shadow and wireframe support][4367]
- [add AnimationPlayer component only on scene roots that are also animation roots][4417]
- [Fix loading non-TriangleList meshes without normals in gltf loader][4376]
- [gltf-loader: disable backface culling if material is double-sided][4270]
- [Fix glTF perspective camera projection][4006]
- [fix mul_vec3 transformation order: should be scale -> rotate -> translate][3811]

[2154]: https://github.com/bevyengine/bevy/pull/2154
[2382]: https://github.com/bevyengine/bevy/pull/2382
[2414]: https://github.com/bevyengine/bevy/pull/2414
[2434]: https://github.com/bevyengine/bevy/pull/2434
[2441]: https://github.com/bevyengine/bevy/pull/2441
[2636]: https://github.com/bevyengine/bevy/pull/2636
[2713]: https://github.com/bevyengine/bevy/pull/2713
[2765]: https://github.com/bevyengine/bevy/pull/2765
[2838]: https://github.com/bevyengine/bevy/pull/2838
[2889]: https://github.com/bevyengine/bevy/pull/2889
[2915]: https://github.com/bevyengine/bevy/pull/2915
[2923]: https://github.com/bevyengine/bevy/pull/2923
[2935]: https://github.com/bevyengine/bevy/pull/2935
[2947]: https://github.com/bevyengine/bevy/pull/2947
[2960]: https://github.com/bevyengine/bevy/pull/2960
[2969]: https://github.com/bevyengine/bevy/pull/2969
[2982]: https://github.com/bevyengine/bevy/pull/2982
[2996]: https://github.com/bevyengine/bevy/pull/2996
[3045]: https://github.com/bevyengine/bevy/pull/3045
[3048]: https://github.com/bevyengine/bevy/pull/3048
[3054]: https://github.com/bevyengine/bevy/pull/3054
[3065]: https://github.com/bevyengine/bevy/pull/3065
[3079]: https://github.com/bevyengine/bevy/pull/3079
[3090]: https://github.com/bevyengine/bevy/pull/3090
[3107]: https://github.com/bevyengine/bevy/pull/3107
[3302]: https://github.com/bevyengine/bevy/pull/3302
[3328]: https://github.com/bevyengine/bevy/pull/3328
[3412]: https://github.com/bevyengine/bevy/pull/3412
[3463]: https://github.com/bevyengine/bevy/pull/3463
[3535]: https://github.com/bevyengine/bevy/pull/3535
[3560]: https://github.com/bevyengine/bevy/pull/3560
[3565]: https://github.com/bevyengine/bevy/pull/3565
[3603]: https://github.com/bevyengine/bevy/pull/3603
[3617]: https://github.com/bevyengine/bevy/pull/3617
[3633]: https://github.com/bevyengine/bevy/pull/3633
[3635]: https://github.com/bevyengine/bevy/pull/3635
[3643]: https://github.com/bevyengine/bevy/pull/3643
[3656]: https://github.com/bevyengine/bevy/pull/3656
[3675]: https://github.com/bevyengine/bevy/pull/3675
[3685]: https://github.com/bevyengine/bevy/pull/3685
[3696]: https://github.com/bevyengine/bevy/pull/3696
[3697]: https://github.com/bevyengine/bevy/pull/3697
[3698]: https://github.com/bevyengine/bevy/pull/3698
[3711]: https://github.com/bevyengine/bevy/pull/3711
[3724]: https://github.com/bevyengine/bevy/pull/3724
[3739]: https://github.com/bevyengine/bevy/pull/3739
[3741]: https://github.com/bevyengine/bevy/pull/3741
[3751]: https://github.com/bevyengine/bevy/pull/3751
[3772]: https://github.com/bevyengine/bevy/pull/3772
[3785]: https://github.com/bevyengine/bevy/pull/3785
[3795]: https://github.com/bevyengine/bevy/pull/3795
[3803]: https://github.com/bevyengine/bevy/pull/3803
[3811]: https://github.com/bevyengine/bevy/pull/3811
[3812]: https://github.com/bevyengine/bevy/pull/3812
[3817]: https://github.com/bevyengine/bevy/pull/3817
[3849]: https://github.com/bevyengine/bevy/pull/3849
[3884]: https://github.com/bevyengine/bevy/pull/3884
[3885]: https://github.com/bevyengine/bevy/pull/3885
[3912]: https://github.com/bevyengine/bevy/pull/3912
[3916]: https://github.com/bevyengine/bevy/pull/3916
[3917]: https://github.com/bevyengine/bevy/pull/3917
[3923]: https://github.com/bevyengine/bevy/pull/3923
[3926]: https://github.com/bevyengine/bevy/pull/3926
[3930]: https://github.com/bevyengine/bevy/pull/3930
[3931]: https://github.com/bevyengine/bevy/pull/3931
[3941]: https://github.com/bevyengine/bevy/pull/3941
[3945]: https://github.com/bevyengine/bevy/pull/3945
[3948]: https://github.com/bevyengine/bevy/pull/3948
[3952]: https://github.com/bevyengine/bevy/pull/3952
[3958]: https://github.com/bevyengine/bevy/pull/3958
[3959]: https://github.com/bevyengine/bevy/pull/3959
[3963]: https://github.com/bevyengine/bevy/pull/3963
[3966]: https://github.com/bevyengine/bevy/pull/3966
[3968]: https://github.com/bevyengine/bevy/pull/3968
[3974]: https://github.com/bevyengine/bevy/pull/3974
[3976]: https://github.com/bevyengine/bevy/pull/3976
[3979]: https://github.com/bevyengine/bevy/pull/3979
[3981]: https://github.com/bevyengine/bevy/pull/3981
[3982]: https://github.com/bevyengine/bevy/pull/3982
[3984]: https://github.com/bevyengine/bevy/pull/3984
[3989]: https://github.com/bevyengine/bevy/pull/3989
[4006]: https://github.com/bevyengine/bevy/pull/4006
[4012]: https://github.com/bevyengine/bevy/pull/4012
[4015]: https://github.com/bevyengine/bevy/pull/4015
[4035]: https://github.com/bevyengine/bevy/pull/4035
[4047]: https://github.com/bevyengine/bevy/pull/4047
[4055]: https://github.com/bevyengine/bevy/pull/4055
[4056]: https://github.com/bevyengine/bevy/pull/4056
[4063]: https://github.com/bevyengine/bevy/pull/4063
[4071]: https://github.com/bevyengine/bevy/pull/4071
[4079]: https://github.com/bevyengine/bevy/pull/4079
[4086]: https://github.com/bevyengine/bevy/pull/4086
[4088]: https://github.com/bevyengine/bevy/pull/4088
[4092]: https://github.com/bevyengine/bevy/pull/4092
[4093]: https://github.com/bevyengine/bevy/pull/4093
[4096]: https://github.com/bevyengine/bevy/pull/4096
[4114]: https://github.com/bevyengine/bevy/pull/4114
[4115]: https://github.com/bevyengine/bevy/pull/4115
[4119]: https://github.com/bevyengine/bevy/pull/4119
[4126]: https://github.com/bevyengine/bevy/pull/4126
[4148]: https://github.com/bevyengine/bevy/pull/4148
[4168]: https://github.com/bevyengine/bevy/pull/4168
[4169]: https://github.com/bevyengine/bevy/pull/4169
[4170]: https://github.com/bevyengine/bevy/pull/4170
[4179]: https://github.com/bevyengine/bevy/pull/4179
[4180]: https://github.com/bevyengine/bevy/pull/4180
[4181]: https://github.com/bevyengine/bevy/pull/4181
[4182]: https://github.com/bevyengine/bevy/pull/4182
[4183]: https://github.com/bevyengine/bevy/pull/4183
[4194]: https://github.com/bevyengine/bevy/pull/4194
[4224]: https://github.com/bevyengine/bevy/pull/4224
[4225]: https://github.com/bevyengine/bevy/pull/4225
[4238]: https://github.com/bevyengine/bevy/pull/4238
[4243]: https://github.com/bevyengine/bevy/pull/4243
[4246]: https://github.com/bevyengine/bevy/pull/4246
[4250]: https://github.com/bevyengine/bevy/pull/4250
[4270]: https://github.com/bevyengine/bevy/pull/4270
[4298]: https://github.com/bevyengine/bevy/pull/4298
[4310]: https://github.com/bevyengine/bevy/pull/4310
[4328]: https://github.com/bevyengine/bevy/pull/4328
[4340]: https://github.com/bevyengine/bevy/pull/4340
[4347]: https://github.com/bevyengine/bevy/pull/4347
[4361]: https://github.com/bevyengine/bevy/pull/4361
[4367]: https://github.com/bevyengine/bevy/pull/4367
[4375]: https://github.com/bevyengine/bevy/pull/4375
[4376]: https://github.com/bevyengine/bevy/pull/4376
[4383]: https://github.com/bevyengine/bevy/pull/4383
[4396]: https://github.com/bevyengine/bevy/pull/4396
[4399]: https://github.com/bevyengine/bevy/pull/4399
[4400]: https://github.com/bevyengine/bevy/pull/4400
[4417]: https://github.com/bevyengine/bevy/pull/4417
[4427]: https://github.com/bevyengine/bevy/pull/4427
[4428]: https://github.com/bevyengine/bevy/pull/4428
[4433]: https://github.com/bevyengine/bevy/pull/4433
[4435]: https://github.com/bevyengine/bevy/pull/4435

## Version 0.6.0 (2022-01-08)

### Added

- [New Renderer][3175]
- [Clustered forward rendering][3153]
- [Frustum culling][2861]
- [Sprite Batching][3060]
- [Materials and MaterialPlugin][3428]
- [2D Meshes and Materials][3460]
- [WebGL2 support][3039]
- [Pipeline Specialization, Shader Assets, and Shader Preprocessing][3031]
- [Modular Rendering][2831]
- [Directional light and shadow][c6]
- [Directional light][2112]
- [Use the infinite reverse right-handed perspective projection][2543]
- [Implement and require `#[derive(Component)]` on all component structs][2254]
- [Shader Imports. Decouple Mesh logic from PBR][3137]
- [Add support for opaque, alpha mask, and alpha blend modes][3072]
- [bevy_gltf: Load light names from gltf][3553]
- [bevy_gltf: Add support for loading lights][3506]
- [Spherical Area Lights][1901]
- [Shader Processor: process imported shader][3290]
- [Add support for not casting/receiving shadows][2726]
- [Add support for configurable shadow map sizes][2700]
- [Implement the `Overflow::Hidden` style property for UI][3296]
- [SystemState][2283]
- [Add a method `iter_combinations` on query to iterate over combinations of query results][1763]
- [Add FromReflect trait to convert dynamic types to concrete types][1395]
- [More pipelined-rendering shader examples][3041]
- [Configurable wgpu features/limits priority][3452]
- [Cargo feature for bevy UI][3546]
- [Spherical area lights example][3498]
- [Implement ReflectValue serialization for Duration][3318]
- [bevy_ui: register Overflow type][3443]
- [Add Visibility component to UI][3426]
- [Implement non-indexed mesh rendering][3415]
- [add tracing spans for parallel executor and system overhead][3416]
- [RemoveChildren command][1925]
- [report shader processing errors in `RenderPipelineCache`][3289]
- [enable Webgl2 optimisation in pbr under feature][3291]
- [Implement Sub-App Labels][2695]
- [Added `set_cursor_icon(...)` to `Window`][3395]
- [Support topologies other than TriangleList][3349]
- [Add an example 'showcasing' using multiple windows][3367]
- [Add an example to draw a rectangle][2957]
- [Added set_scissor_rect to tracked render pass.][3320]
- [Add RenderWorld to Extract step][2555]
- [re-export ClearPassNode][3336]
- [add default standard material in PbrBundle][3325]
- [add methods to get reads and writes of `Access<T>`][3166]
- [Add despawn_children][2903]
- [More Bevy ECS schedule spans][3281]
- [Added transparency to window builder][3105]
- [Add Gamepads resource][3257]
- [Add support for #else for shader defs][3206]
- [Implement iter() for mutable Queries][2305]
- [add shadows in examples][3201]
- [Added missing wgpu image render resources.][3171]
- [Per-light toggleable shadow mapping][3126]
- [Support nested shader defs][3113]
- [use bytemuck crate instead of Byteable trait][2183]
- [`iter_mut()` for Assets type][3118]
- [EntityRenderCommand and PhaseItemRenderCommand][3111]
- [add position to WindowDescriptor][3070]
- [Add System Command apply and RenderGraph node spans][3069]
- [Support for normal maps including from glTF models][2741]
- [MSAA example][3049]
- [Add MSAA to new renderer][3042]
- [Add support for IndexFormat::Uint16][2990]
- [Apply labels to wgpu resources for improved debugging/profiling][2912]
- [Add tracing spans around render subapp and stages][2907]
- [Add set_stencil_reference to TrackedRenderPass][2885]
- [Add despawn_recursive to EntityMut][2855]
- [Add trace_tracy feature for Tracy profiling][2832]
- [Expose wgpu's StencilOperation with bevy][2819]
- [add get_single variant][2793]
- [Add builder methods to Transform][2778]
- [add get_history function to Diagnostic][2772]
- [Add convenience methods for checking a set of inputs][2760]
- [Add error messages for the spooky insertions][2581]
- [Add Deref implementation for ComputePipeline][2759]
- [Derive thiserror::Error for HexColorError][2740]
- [Spawn specific entities: spawn or insert operations, refactor spawn internals, world clearing][2673]
- [Add ClearColor Resource to Pipelined Renderer][2631]
- [remove_component for ReflectComponent][2682]
- [Added ComputePipelineDescriptor][2628]
- [Added StorageTextureAccess to the exposed wgpu API][2614]
- [Add sprite atlases into the new renderer.][2560]
- [Log adapter info on initialization][2542]
- [Add feature flag to enable wasm for bevy_audio][2397]
- [Allow `Option<NonSend<T>>` and `Option<NonSendMut<T>>` as SystemParam][2345]
- [Added helpful adders for systemsets][2366]
- [Derive Clone for Time][2360]
- [Implement Clone for Fetches][2641]
- [Implement IntoSystemDescriptor for SystemDescriptor][2718]
- [implement DetectChanges for NonSendMut][2326]
- [Log errors when loading textures from a gltf file][2260]
- [expose texture/image conversions as From/TryFrom][2175]
- [[ecs] implement is_empty for queries][2271]
- [Add audio to ios example][1007]
- [Example showing how to use AsyncComputeTaskPool and Tasks][2180]
- [Expose set_changed() on ResMut and Mut][2208]
- [Impl AsRef+AsMut for Res, ResMut, and Mut][2189]
- [Add exit_on_esc_system to examples with window][2121]
- [Implement rotation for Text2d][2084]
- [Mesh vertex attributes for skinning and animation][1831]
- [load zeroed UVs as fallback in gltf loader][1803]
- [Implement direct mutable dereferencing][2100]
- [add a span for frames][2053]
- [Add an alias mouse position -> cursor position][2038]
- [Adding `WorldQuery` for `WithBundle`][2024]
- [Automatic System Spans][2033]
- [Add system sets and run criteria example][1909]
- [EnumVariantMeta derive][1972]
- [Added TryFrom for VertexAttributeValues][1963]
- [add render_to_texture example][1927]
- [Added example of entity sorting by components][1817]
- [calculate flat normals for mesh if missing][1808]
- [Add animate shaders example][1765]
- [examples on how to tests systems][1714]
- [Add a UV sphere implementation][1887]
- [Add additional vertex formats][1878]
- [gltf-loader: support data url for images][1828]
- [glTF: added color attribute support][1775]
- [Add synonyms for transform relative vectors][1667]

### Changed

- [Relicense Bevy under the dual MIT or Apache-2.0 license][2509]
- [[ecs] Improve `Commands` performance][2332]
- [Merge AppBuilder into App][2531]
- [Use a special first depth slice for clustered forward rendering][3545]
- [Add a separate ClearPass][3209]
- [bevy_pbr2: Improve lighting units and documentation][2704]
- [gltf loader: do not use the taskpool for only one task][3577]
- [System Param Lifetime Split][2605]
- [Optional `.system`][2398]
- [Optional `.system()`, part 2][2403]
- [Optional `.system()`, part 3][2422]
- [Optional `.system()`, part 4 (run criteria)][2431]
- [Optional `.system()`, part 6 (chaining)][2494]
- [Make the `iter_combinators` examples prettier][3075]
- [Remove dead anchor.rs code][3551]
- [gltf: load textures asynchronously using io task pool][1767]
- [Use fully-qualified type names in Label derive.][3544]
- [Remove Bytes, FromBytes, Labels, EntityLabels][3521]
- [StorageType parameter removed from ComponentDescriptor::new_resource][3495]
- [remove dead code: ShaderDefs derive][3490]
- [Enable Msaa for webgl by default][3489]
- [Renamed Entity::new to Entity::from_raw][3465]
- [bevy::scene::Entity renamed to bevy::scene::DynamicEntity.][3448]
- [make `sub_app` return an `&App` and add `sub_app_mut() -> &mut App`][3309]
- [use ogg by default instead of mp3][3421]
- [enable `wasm-bindgen` feature on gilrs][3420]
- [Use EventWriter for gilrs_system][3413]
- [Add some of the missing methods to `TrackedRenderPass`][3401]
- [Only bevy_render depends directly on wgpu][3393]
- [Update wgpu to 0.12 and naga to 0.8][3375]
- [Improved bevymark: no bouncing offscreen and spawn waves from CLI][3364]
- [Rename render UiSystem to RenderUiSystem][3371]
- [Use updated window size in bevymark example][3335]
- [Enable trace feature for subfeatures using it][3337]
- [Schedule gilrs system before input systems][2989]
- [Rename fixed timestep state and add a test][3260]
- [Port bevy_ui to pipelined-rendering][2653]
- [update wireframe rendering to new renderer][3193]
- [Allow `String` and `&String` as `Id` for `AssetServer.get_handle(id)`][3280]
- [Ported WgpuOptions to new renderer][3282]
- [Down with the system!][2496]
- [Update dependencies `ron` `winit`& fix `cargo-deny` lists][3244]
- [Improve contributors example quality][3258]
- [Expose command encoders][3271]
- [Made Time::time_since_startup return from last tick.][3264]
- [Default image used in PipelinedSpriteBundle to be able to render without loading a texture][3270]
- [make texture from sprite pipeline filterable][3236]
- [iOS: replace cargo-lipo, and update for new macOS][3109]
- [increase light intensity in pbr example][3182]
- [Faster gltf loader][3189]
- [Use crevice std140_size_static everywhere][3168]
- [replace matrix swizzles in pbr shader with index accesses][3122]
- [Disable default features from `bevy_asset` and `bevy_ecs`][3097]
- [Update tracing-subscriber requirement from 0.2.22 to 0.3.1][3076]
- [Update vendored Crevice to 0.8.0 + PR for arrays][3059]
- [change texture atlas sprite indexing to usize][2887]
- [Update derive(DynamicPlugin) to edition 2021][3038]
- [Update to edition 2021 on master][3028]
- [Add entity ID to expect() message][2943]
- [Use RenderQueue in BufferVec][2847]
- [removed unused RenderResourceId and SwapChainFrame][2890]
- [Unique WorldId][2827]
- [add_texture returns index to texture][2864]
- [Update hexasphere requirement from 4.0.0 to 5.0.0][2880]
- [enable change detection for hierarchy maintenance][2411]
- [Make events reuse buffers][2850]
- [Replace `.insert_resource(T::default())` calls with `init_resource::<T>()`][2807]
- [Improve many sprites example][2785]
- [Update glam requirement from 0.17.3 to 0.18.0][2748]
- [update ndk-glue to 0.4][2684]
- [Remove Need for Sprite Size Sync System][2632]
- [Pipelined separate shadow vertex shader][2727]
- [Sub app label changes][2717]
- [Use Explicit Names for Flex Direction][2672]
- [Make default near plane more sensible at 0.1][2703]
- [Reduce visibility of various types and fields][2690]
- [Cleanup FromResources][2601]
- [Better error message for unsupported shader features Fixes #869][2598]
- [Change definition of `ScheduleRunnerPlugin`][2606]
- [Re-implement Automatic Sprite Sizing][2613]
- [Remove with bundle filter][2623]
- [Remove bevy_dynamic_plugin as a default][2578]
- [Port bevy_gltf to pipelined-rendering][2537]
- [Bump notify to 5.0.0-pre.11][2564]
- [Add 's (state) lifetime to `Fetch`][2515]
- [move bevy_core_pipeline to its own plugin][2552]
- [Refactor ECS to reduce the dependency on a 1-to-1 mapping between components and real rust types][2490]
- [Inline world get][2520]
- [Dedupe move logic in remove_bundle and remove_bundle_intersection][2521]
- [remove .system from pipelined code][2538]
- [Scale normal bias by texel size][c26]
- [Make Remove Command's fields public][2449]
- [bevy_utils: Re-introduce `with_capacity()`.][2393]
- [Update rodio requirement from 0.13 to 0.14][2244]
- [Optimize Events::extend and impl std::iter::Extend][2207]
- [Bump winit to 0.25][2186]
- [Improve legibility of RunOnce::run_unsafe param][2181]
- [Update gltf requirement from 0.15.2 to 0.16.0][2196]
- [Move to smallvec v1.6][2074]
- [Update rectangle-pack requirement from 0.3 to 0.4][2086]
- [Make Commands public?][2034]
- [Monomorphize various things][1914]
- [Detect camera projection changes][2015]
- [support assets of any size][1997]
- [Separate Query filter access from fetch access during initial evaluation][1977]
- [Provide better error message when missing a render backend][1965]
- [par_for_each: split batches when iterating on a sparse query][1945]
- [Allow deriving `SystemParam` on private types][1936]
- [Angle bracket annotated types to support generics][1919]
- [More detailed errors when resource not found][1864]
- [Moved events to ECS][1823]
- [Use a sorted Map for vertex buffer attributes][1796]
- [Error message improvements for shader compilation/gltf loading][1786]
- [Rename Light => PointLight and remove unused properties][1778]
- [Override size_hint for all Iterators and add ExactSizeIterator where applicable][1734]
- [Change breakout to use fixed timestamp][1541]

### Fixed

- [Fix shadows for non-TriangleLists][3581]
- [Fix error message for the `Component` macro's `component` `storage` attribute.][3534]
- [do not add plugin ExtractComponentPlugin twice for StandardMaterial][3502]
- [load spirv using correct API][3466]
- [fix shader compilation error reporting for non-wgsl shaders][3441]
- [bevy_ui: Check clip when handling interactions][3461]
- [crevice derive macro: fix path to render_resource when importing from bevy][3438]
- [fix parenting of scenes][2410]
- [Do not panic on failed setting of GameOver state in AlienCakeAddict][3411]
- [Fix minimization crash because of cluster updates.][3369]
- [Fix custom mesh pipelines][3381]
- [Fix hierarchy example panic][3378]
- [Fix double drop in BlobVec::replace_unchecked (#2597)][2848]
- [Remove vestigial derives][3343]
- [Fix crash with disabled winit][3330]
- [Fix clustering for orthographic projections][3316]
- [Run a clear pass on Windows without any Views][3304]
- [Remove some superfluous unsafe code][3297]
- [clearpass: also clear views without depth (2d)][3286]
- [Check for NaN in `Camera::world_to_screen()`][3268]
- [Fix sprite hot reloading in new renderer][3207]
- [Fix path used by macro not considering that we can use a sub-crate][3178]
- [Fix torus normals][3549]
- [enable alpha mode for textures materials that are transparent][3202]
- [fix calls to as_rgba_linear][3200]
- [Fix shadow logic][3186]
- [fix: as_rgba_linear used wrong variant][3192]
- [Fix MIME type support for glTF buffer Data URIs][3101]
- [Remove wasm audio feature flag for 2021][3000]
- [use correct size of pixel instead of 4][2977]
- [Fix custom_shader_pipelined example shader][2992]
- [Fix scale factor for cursor position][2932]
- [fix window resize after wgpu 0.11 upgrade][2953]
- [Fix unsound lifetime annotation on `Query::get_component`][2964]
- [Remove double Events::update in bevy-gilrs][2894]
- [Fix bevy_ecs::schedule::executor_parallel::system span management][2905]
- [Avoid some format! into immediate format!][2913]
- [Fix panic on is_resource_* calls (#2828)][2863]
- [Fix window size change panic][2858]
- [fix `Default` implementation of `Image` so that size and data match][2833]
- [Fix scale_factor_override in the winit backend][2784]
- [Fix breakout example scoreboard][2770]
- [Fix `Option<NonSend<T>>` and `Option<NonSendMut<T>>`][2757]
- [fix missing paths in ECS SystemParam derive macro v2][2550]
- [Add missing bytemuck feature][2625]
- [Update EntityMut's location in push_children() and insert_children()][2604]
- [Fixed issue with how texture arrays were uploaded with write_texture.][c24]
- [Don't update when suspended to avoid GPU use on iOS.][2482]
- [update archetypes for run criterias][2177]
- [Fix AssetServer::get_asset_loader deadlock][2395]
- [Fix unsetting RenderLayers bit in without fn][2409]
- [Fix view vector in pbr frag to work in ortho][2370]
- [Fixes Timer Precision Error Causing Panic][2362]
- [[assets] Fix `AssetServer::get_handle_path`][2310]
- [Fix bad bounds for NonSend SystemParams][2325]
- [Add minimum sizes to textures to prevent crash][2300]
- [[assets] set LoadState properly and more testing!][2226]
- [[assets] properly set `LoadState` with invalid asset extension][2318]
- [Fix Bevy crashing if no audio device is found][2269]
- [Fixes dropping empty BlobVec][2295]
- [[assets] fix Assets being set as 'changed' each frame][2280]
- [drop overwritten component data on double insert][2227]
- [Despawn with children doesn't need to remove entities from parents children when parents are also removed][2278]
- [reduce tricky unsafety and simplify table structure][2221]
- [Use bevy_reflect as path in case of no direct references][1875]
- [Fix Events::<drain/clear> bug][2206]
- [small ecs cleanup and remove_bundle drop bugfix][2172]
- [Fix PBR regression for unlit materials][2197]
- [prevent memory leak when dropping ParallelSystemContainer][2176]
- [fix diagnostic length for asset count][2165]
- [Fixes incorrect `PipelineCompiler::compile_pipeline()` step_mode][2126]
- [Asset re-loading while it's being deleted][2011]
- [Bevy derives handling generics in impl definitions.][2044]
- [Fix unsoundness in `Query::for_each_mut`][2045]
- [Fix mesh with no vertex attributes causing panic][2036]
- [Fix alien_cake_addict: cake should not be at height of player's location][1954]
- [fix memory size for PointLightBundle][1940]
- [Fix unsoundness in query component access][1929]
- [fixing compilation error on macos aarch64][1905]
- [Fix SystemParam handling of Commands][1899]
- [Fix IcoSphere UV coordinates][1871]
- [fix 'attempted to subtract with overflow' for State::inactives][1668]

[1007]: https://github.com/bevyengine/bevy/pull/1007
[1395]: https://github.com/bevyengine/bevy/pull/1395
[1541]: https://github.com/bevyengine/bevy/pull/1541
[1667]: https://github.com/bevyengine/bevy/pull/1667
[1668]: https://github.com/bevyengine/bevy/pull/1668
[1714]: https://github.com/bevyengine/bevy/pull/1714
[1734]: https://github.com/bevyengine/bevy/pull/1734
[1763]: https://github.com/bevyengine/bevy/pull/1763
[1765]: https://github.com/bevyengine/bevy/pull/1765
[1767]: https://github.com/bevyengine/bevy/pull/1767
[1775]: https://github.com/bevyengine/bevy/pull/1775
[1778]: https://github.com/bevyengine/bevy/pull/1778
[1786]: https://github.com/bevyengine/bevy/pull/1786
[1796]: https://github.com/bevyengine/bevy/pull/1796
[1803]: https://github.com/bevyengine/bevy/pull/1803
[1808]: https://github.com/bevyengine/bevy/pull/1808
[1817]: https://github.com/bevyengine/bevy/pull/1817
[1823]: https://github.com/bevyengine/bevy/pull/1823
[1828]: https://github.com/bevyengine/bevy/pull/1828
[1831]: https://github.com/bevyengine/bevy/pull/1831
[1864]: https://github.com/bevyengine/bevy/pull/1864
[1871]: https://github.com/bevyengine/bevy/pull/1871
[1875]: https://github.com/bevyengine/bevy/pull/1875
[1878]: https://github.com/bevyengine/bevy/pull/1878
[1887]: https://github.com/bevyengine/bevy/pull/1887
[1899]: https://github.com/bevyengine/bevy/pull/1899
[1901]: https://github.com/bevyengine/bevy/pull/1901
[1905]: https://github.com/bevyengine/bevy/pull/1905
[1909]: https://github.com/bevyengine/bevy/pull/1909
[1914]: https://github.com/bevyengine/bevy/pull/1914
[1919]: https://github.com/bevyengine/bevy/pull/1919
[1925]: https://github.com/bevyengine/bevy/pull/1925
[1927]: https://github.com/bevyengine/bevy/pull/1927
[1929]: https://github.com/bevyengine/bevy/pull/1929
[1936]: https://github.com/bevyengine/bevy/pull/1936
[1940]: https://github.com/bevyengine/bevy/pull/1940
[1945]: https://github.com/bevyengine/bevy/pull/1945
[1954]: https://github.com/bevyengine/bevy/pull/1954
[1963]: https://github.com/bevyengine/bevy/pull/1963
[1965]: https://github.com/bevyengine/bevy/pull/1965
[1972]: https://github.com/bevyengine/bevy/pull/1972
[1977]: https://github.com/bevyengine/bevy/pull/1977
[1997]: https://github.com/bevyengine/bevy/pull/1997
[2011]: https://github.com/bevyengine/bevy/pull/2011
[2015]: https://github.com/bevyengine/bevy/pull/2015
[2024]: https://github.com/bevyengine/bevy/pull/2024
[2033]: https://github.com/bevyengine/bevy/pull/2033
[2034]: https://github.com/bevyengine/bevy/pull/2034
[2036]: https://github.com/bevyengine/bevy/pull/2036
[2038]: https://github.com/bevyengine/bevy/pull/2038
[2044]: https://github.com/bevyengine/bevy/pull/2044
[2045]: https://github.com/bevyengine/bevy/pull/2045
[2053]: https://github.com/bevyengine/bevy/pull/2053
[2074]: https://github.com/bevyengine/bevy/pull/2074
[2084]: https://github.com/bevyengine/bevy/pull/2084
[2086]: https://github.com/bevyengine/bevy/pull/2086
[2100]: https://github.com/bevyengine/bevy/pull/2100
[2112]: https://github.com/bevyengine/bevy/pull/2112
[2121]: https://github.com/bevyengine/bevy/pull/2121
[2126]: https://github.com/bevyengine/bevy/pull/2126
[2165]: https://github.com/bevyengine/bevy/pull/2165
[2172]: https://github.com/bevyengine/bevy/pull/2172
[2175]: https://github.com/bevyengine/bevy/pull/2175
[2176]: https://github.com/bevyengine/bevy/pull/2176
[2177]: https://github.com/bevyengine/bevy/pull/2177
[2180]: https://github.com/bevyengine/bevy/pull/2180
[2181]: https://github.com/bevyengine/bevy/pull/2181
[2183]: https://github.com/bevyengine/bevy/pull/2183
[2186]: https://github.com/bevyengine/bevy/pull/2186
[2189]: https://github.com/bevyengine/bevy/pull/2189
[2196]: https://github.com/bevyengine/bevy/pull/2196
[2197]: https://github.com/bevyengine/bevy/pull/2197
[2206]: https://github.com/bevyengine/bevy/pull/2206
[2207]: https://github.com/bevyengine/bevy/pull/2207
[2208]: https://github.com/bevyengine/bevy/pull/2208
[2221]: https://github.com/bevyengine/bevy/pull/2221
[2226]: https://github.com/bevyengine/bevy/pull/2226
[2227]: https://github.com/bevyengine/bevy/pull/2227
[2244]: https://github.com/bevyengine/bevy/pull/2244
[2254]: https://github.com/bevyengine/bevy/pull/2254
[2260]: https://github.com/bevyengine/bevy/pull/2260
[2269]: https://github.com/bevyengine/bevy/pull/2269
[2271]: https://github.com/bevyengine/bevy/pull/2271
[2278]: https://github.com/bevyengine/bevy/pull/2278
[2280]: https://github.com/bevyengine/bevy/pull/2280
[2283]: https://github.com/bevyengine/bevy/pull/2283
[2295]: https://github.com/bevyengine/bevy/pull/2295
[2300]: https://github.com/bevyengine/bevy/pull/2300
[2305]: https://github.com/bevyengine/bevy/pull/2305
[2310]: https://github.com/bevyengine/bevy/pull/2310
[2318]: https://github.com/bevyengine/bevy/pull/2318
[2325]: https://github.com/bevyengine/bevy/pull/2325
[2326]: https://github.com/bevyengine/bevy/pull/2326
[2332]: https://github.com/bevyengine/bevy/pull/2332
[2345]: https://github.com/bevyengine/bevy/pull/2345
[2360]: https://github.com/bevyengine/bevy/pull/2360
[2362]: https://github.com/bevyengine/bevy/pull/2362
[2366]: https://github.com/bevyengine/bevy/pull/2366
[2370]: https://github.com/bevyengine/bevy/pull/2370
[2393]: https://github.com/bevyengine/bevy/pull/2393
[2395]: https://github.com/bevyengine/bevy/pull/2395
[2397]: https://github.com/bevyengine/bevy/pull/2397
[2398]: https://github.com/bevyengine/bevy/pull/2398
[2403]: https://github.com/bevyengine/bevy/pull/2403
[2409]: https://github.com/bevyengine/bevy/pull/2409
[2410]: https://github.com/bevyengine/bevy/pull/2410
[2411]: https://github.com/bevyengine/bevy/pull/2411
[2422]: https://github.com/bevyengine/bevy/pull/2422
[2431]: https://github.com/bevyengine/bevy/pull/2431
[2449]: https://github.com/bevyengine/bevy/pull/2449
[2482]: https://github.com/bevyengine/bevy/pull/2482
[2490]: https://github.com/bevyengine/bevy/pull/2490
[2494]: https://github.com/bevyengine/bevy/pull/2494
[2496]: https://github.com/bevyengine/bevy/pull/2496
[2509]: https://github.com/bevyengine/bevy/pull/2509
[2515]: https://github.com/bevyengine/bevy/pull/2515
[2520]: https://github.com/bevyengine/bevy/pull/2520
[2521]: https://github.com/bevyengine/bevy/pull/2521
[2531]: https://github.com/bevyengine/bevy/pull/2531
[2537]: https://github.com/bevyengine/bevy/pull/2537
[2538]: https://github.com/bevyengine/bevy/pull/2538
[2542]: https://github.com/bevyengine/bevy/pull/2542
[2543]: https://github.com/bevyengine/bevy/pull/2543
[2550]: https://github.com/bevyengine/bevy/pull/2550
[2552]: https://github.com/bevyengine/bevy/pull/2552
[2555]: https://github.com/bevyengine/bevy/pull/2555
[2560]: https://github.com/bevyengine/bevy/pull/2560
[2564]: https://github.com/bevyengine/bevy/pull/2564
[2578]: https://github.com/bevyengine/bevy/pull/2578
[2581]: https://github.com/bevyengine/bevy/pull/2581
[2598]: https://github.com/bevyengine/bevy/pull/2598
[2601]: https://github.com/bevyengine/bevy/pull/2601
[2604]: https://github.com/bevyengine/bevy/pull/2604
[2605]: https://github.com/bevyengine/bevy/pull/2605
[2606]: https://github.com/bevyengine/bevy/pull/2606
[2613]: https://github.com/bevyengine/bevy/pull/2613
[2614]: https://github.com/bevyengine/bevy/pull/2614
[2623]: https://github.com/bevyengine/bevy/pull/2623
[2625]: https://github.com/bevyengine/bevy/pull/2625
[2628]: https://github.com/bevyengine/bevy/pull/2628
[2631]: https://github.com/bevyengine/bevy/pull/2631
[2632]: https://github.com/bevyengine/bevy/pull/2632
[2641]: https://github.com/bevyengine/bevy/pull/2641
[2653]: https://github.com/bevyengine/bevy/pull/2653
[2672]: https://github.com/bevyengine/bevy/pull/2672
[2673]: https://github.com/bevyengine/bevy/pull/2673
[2682]: https://github.com/bevyengine/bevy/pull/2682
[2684]: https://github.com/bevyengine/bevy/pull/2684
[2690]: https://github.com/bevyengine/bevy/pull/2690
[2695]: https://github.com/bevyengine/bevy/pull/2695
[2700]: https://github.com/bevyengine/bevy/pull/2700
[2703]: https://github.com/bevyengine/bevy/pull/2703
[2704]: https://github.com/bevyengine/bevy/pull/2704
[2717]: https://github.com/bevyengine/bevy/pull/2717
[2718]: https://github.com/bevyengine/bevy/pull/2718
[2726]: https://github.com/bevyengine/bevy/pull/2726
[2727]: https://github.com/bevyengine/bevy/pull/2727
[2740]: https://github.com/bevyengine/bevy/pull/2740
[2741]: https://github.com/bevyengine/bevy/pull/2741
[2748]: https://github.com/bevyengine/bevy/pull/2748
[2757]: https://github.com/bevyengine/bevy/pull/2757
[2759]: https://github.com/bevyengine/bevy/pull/2759
[2760]: https://github.com/bevyengine/bevy/pull/2760
[2770]: https://github.com/bevyengine/bevy/pull/2770
[2772]: https://github.com/bevyengine/bevy/pull/2772
[2778]: https://github.com/bevyengine/bevy/pull/2778
[2784]: https://github.com/bevyengine/bevy/pull/2784
[2785]: https://github.com/bevyengine/bevy/pull/2785
[2793]: https://github.com/bevyengine/bevy/pull/2793
[2807]: https://github.com/bevyengine/bevy/pull/2807
[2819]: https://github.com/bevyengine/bevy/pull/2819
[2827]: https://github.com/bevyengine/bevy/pull/2827
[2831]: https://github.com/bevyengine/bevy/pull/2831
[2832]: https://github.com/bevyengine/bevy/pull/2832
[2833]: https://github.com/bevyengine/bevy/pull/2833
[2847]: https://github.com/bevyengine/bevy/pull/2847
[2848]: https://github.com/bevyengine/bevy/pull/2848
[2850]: https://github.com/bevyengine/bevy/pull/2850
[2855]: https://github.com/bevyengine/bevy/pull/2855
[2858]: https://github.com/bevyengine/bevy/pull/2858
[2861]: https://github.com/bevyengine/bevy/pull/2861
[2863]: https://github.com/bevyengine/bevy/pull/2863
[2864]: https://github.com/bevyengine/bevy/pull/2864
[2880]: https://github.com/bevyengine/bevy/pull/2880
[2885]: https://github.com/bevyengine/bevy/pull/2885
[2887]: https://github.com/bevyengine/bevy/pull/2887
[2890]: https://github.com/bevyengine/bevy/pull/2890
[2894]: https://github.com/bevyengine/bevy/pull/2894
[2903]: https://github.com/bevyengine/bevy/pull/2903
[2905]: https://github.com/bevyengine/bevy/pull/2905
[2907]: https://github.com/bevyengine/bevy/pull/2907
[2912]: https://github.com/bevyengine/bevy/pull/2912
[2913]: https://github.com/bevyengine/bevy/pull/2913
[2932]: https://github.com/bevyengine/bevy/pull/2932
[2943]: https://github.com/bevyengine/bevy/pull/2943
[2953]: https://github.com/bevyengine/bevy/pull/2953
[2957]: https://github.com/bevyengine/bevy/pull/2957
[2964]: https://github.com/bevyengine/bevy/pull/2964
[2977]: https://github.com/bevyengine/bevy/pull/2977
[2989]: https://github.com/bevyengine/bevy/pull/2989
[2990]: https://github.com/bevyengine/bevy/pull/2990
[2992]: https://github.com/bevyengine/bevy/pull/2992
[3000]: https://github.com/bevyengine/bevy/pull/3000
[3028]: https://github.com/bevyengine/bevy/pull/3028
[3031]: https://github.com/bevyengine/bevy/pull/3031
[3038]: https://github.com/bevyengine/bevy/pull/3038
[3039]: https://github.com/bevyengine/bevy/pull/3039
[3041]: https://github.com/bevyengine/bevy/pull/3041
[3042]: https://github.com/bevyengine/bevy/pull/3042
[3049]: https://github.com/bevyengine/bevy/pull/3049
[3059]: https://github.com/bevyengine/bevy/pull/3059
[3060]: https://github.com/bevyengine/bevy/pull/3060
[3069]: https://github.com/bevyengine/bevy/pull/3069
[3070]: https://github.com/bevyengine/bevy/pull/3070
[3072]: https://github.com/bevyengine/bevy/pull/3072
[3075]: https://github.com/bevyengine/bevy/pull/3075
[3076]: https://github.com/bevyengine/bevy/pull/3076
[3097]: https://github.com/bevyengine/bevy/pull/3097
[3101]: https://github.com/bevyengine/bevy/pull/3101
[3105]: https://github.com/bevyengine/bevy/pull/3105
[3109]: https://github.com/bevyengine/bevy/pull/3109
[3111]: https://github.com/bevyengine/bevy/pull/3111
[3113]: https://github.com/bevyengine/bevy/pull/3113
[3118]: https://github.com/bevyengine/bevy/pull/3118
[3122]: https://github.com/bevyengine/bevy/pull/3122
[3126]: https://github.com/bevyengine/bevy/pull/3126
[3137]: https://github.com/bevyengine/bevy/pull/3137
[3153]: https://github.com/bevyengine/bevy/pull/3153
[3166]: https://github.com/bevyengine/bevy/pull/3166
[3168]: https://github.com/bevyengine/bevy/pull/3168
[3171]: https://github.com/bevyengine/bevy/pull/3171
[3175]: https://github.com/bevyengine/bevy/pull/3175
[3178]: https://github.com/bevyengine/bevy/pull/3178
[3182]: https://github.com/bevyengine/bevy/pull/3182
[3186]: https://github.com/bevyengine/bevy/pull/3186
[3189]: https://github.com/bevyengine/bevy/pull/3189
[3192]: https://github.com/bevyengine/bevy/pull/3192
[3193]: https://github.com/bevyengine/bevy/pull/3193
[3200]: https://github.com/bevyengine/bevy/pull/3200
[3201]: https://github.com/bevyengine/bevy/pull/3201
[3202]: https://github.com/bevyengine/bevy/pull/3202
[3206]: https://github.com/bevyengine/bevy/pull/3206
[3207]: https://github.com/bevyengine/bevy/pull/3207
[3209]: https://github.com/bevyengine/bevy/pull/3209
[3236]: https://github.com/bevyengine/bevy/pull/3236
[3244]: https://github.com/bevyengine/bevy/pull/3244
[3257]: https://github.com/bevyengine/bevy/pull/3257
[3258]: https://github.com/bevyengine/bevy/pull/3258
[3260]: https://github.com/bevyengine/bevy/pull/3260
[3264]: https://github.com/bevyengine/bevy/pull/3264
[3268]: https://github.com/bevyengine/bevy/pull/3268
[3270]: https://github.com/bevyengine/bevy/pull/3270
[3271]: https://github.com/bevyengine/bevy/pull/3271
[3280]: https://github.com/bevyengine/bevy/pull/3280
[3281]: https://github.com/bevyengine/bevy/pull/3281
[3282]: https://github.com/bevyengine/bevy/pull/3282
[3286]: https://github.com/bevyengine/bevy/pull/3286
[3289]: https://github.com/bevyengine/bevy/pull/3289
[3290]: https://github.com/bevyengine/bevy/pull/3290
[3291]: https://github.com/bevyengine/bevy/pull/3291
[3296]: https://github.com/bevyengine/bevy/pull/3296
[3297]: https://github.com/bevyengine/bevy/pull/3297
[3304]: https://github.com/bevyengine/bevy/pull/3304
[3309]: https://github.com/bevyengine/bevy/pull/3309
[3316]: https://github.com/bevyengine/bevy/pull/3316
[3318]: https://github.com/bevyengine/bevy/pull/3318
[3320]: https://github.com/bevyengine/bevy/pull/3320
[3325]: https://github.com/bevyengine/bevy/pull/3325
[3330]: https://github.com/bevyengine/bevy/pull/3330
[3335]: https://github.com/bevyengine/bevy/pull/3335
[3336]: https://github.com/bevyengine/bevy/pull/3336
[3337]: https://github.com/bevyengine/bevy/pull/3337
[3343]: https://github.com/bevyengine/bevy/pull/3343
[3349]: https://github.com/bevyengine/bevy/pull/3349
[3364]: https://github.com/bevyengine/bevy/pull/3364
[3367]: https://github.com/bevyengine/bevy/pull/3367
[3369]: https://github.com/bevyengine/bevy/pull/3369
[3371]: https://github.com/bevyengine/bevy/pull/3371
[3375]: https://github.com/bevyengine/bevy/pull/3375
[3378]: https://github.com/bevyengine/bevy/pull/3378
[3381]: https://github.com/bevyengine/bevy/pull/3381
[3393]: https://github.com/bevyengine/bevy/pull/3393
[3395]: https://github.com/bevyengine/bevy/pull/3395
[3401]: https://github.com/bevyengine/bevy/pull/3401
[3411]: https://github.com/bevyengine/bevy/pull/3411
[3413]: https://github.com/bevyengine/bevy/pull/3413
[3415]: https://github.com/bevyengine/bevy/pull/3415
[3416]: https://github.com/bevyengine/bevy/pull/3416
[3420]: https://github.com/bevyengine/bevy/pull/3420
[3421]: https://github.com/bevyengine/bevy/pull/3421
[3426]: https://github.com/bevyengine/bevy/pull/3426
[3428]: https://github.com/bevyengine/bevy/pull/3428
[3438]: https://github.com/bevyengine/bevy/pull/3438
[3441]: https://github.com/bevyengine/bevy/pull/3441
[3443]: https://github.com/bevyengine/bevy/pull/3443
[3448]: https://github.com/bevyengine/bevy/pull/3448
[3452]: https://github.com/bevyengine/bevy/pull/3452
[3460]: https://github.com/bevyengine/bevy/pull/3460
[3461]: https://github.com/bevyengine/bevy/pull/3461
[3465]: https://github.com/bevyengine/bevy/pull/3465
[3466]: https://github.com/bevyengine/bevy/pull/3466
[3489]: https://github.com/bevyengine/bevy/pull/3489
[3490]: https://github.com/bevyengine/bevy/pull/3490
[3495]: https://github.com/bevyengine/bevy/pull/3495
[3498]: https://github.com/bevyengine/bevy/pull/3498
[3502]: https://github.com/bevyengine/bevy/pull/3502
[3506]: https://github.com/bevyengine/bevy/pull/3506
[3521]: https://github.com/bevyengine/bevy/pull/3521
[3534]: https://github.com/bevyengine/bevy/pull/3534
[3544]: https://github.com/bevyengine/bevy/pull/3544
[3545]: https://github.com/bevyengine/bevy/pull/3545
[3546]: https://github.com/bevyengine/bevy/pull/3546
[3549]: https://github.com/bevyengine/bevy/pull/3549
[3551]: https://github.com/bevyengine/bevy/pull/3551
[3553]: https://github.com/bevyengine/bevy/pull/3553
[3577]: https://github.com/bevyengine/bevy/pull/3577
[3581]: https://github.com/bevyengine/bevy/pull/3581
[c6]: https://github.com/cart/bevy/pull/6
[c24]: https://github.com/cart/bevy/pull/24
[c26]: https://github.com/cart/bevy/pull/26

## Version 0.5.0 (2021-04-06)

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
- [`Text2d` render quality][1171]
- [System sets and run criteria v2][1675]
- [System sets and parallel executor v2][1144]
- [Many-to-many system labels][1576]
- [Non-string labels (#1423 continued)][1473]
- [Make `EventReader` a `SystemParam`][1244]
- [Add `EventWriter`][1575]
- [Reliable change detection][1471]
- [Redo State architecture][1424]
- [`Query::get_unique`][1263]
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
- [use `Name` on node when loading a gltf file][1183]
- [GLTF loader: support mipmap filters][1639]
- [Add support for gltf::Material::unlit][1341]
- [Implement `Reflect` for tuples up to length 12][1218]
- [Process Asset File Extensions With Multiple Dots][1277]
- [Update Scene Example to Use scn.ron File][1339]
- [3d game example][1252]
- [Add keyboard modifier example (#1656)][1657]
- [Count number of times a repeating Timer wraps around in a tick][1112]
- [recycle `Timer` refactor to duration.sparkles Add `Stopwatch` struct.][1151]
- [add scene instance entity iteration][1058]
- [Make `Commands` and `World` apis consistent][1703]
- [Add `insert_children` and `push_children` to `EntityMut`][1728]
- [Extend `AppBuilder` api with `add_system_set` and similar methods][1453]
- [add labels and ordering for transform and parent systems in `POST_UPDATE` stage][1456]
- [Explicit execution order ambiguities API][1469]
- [Resolve (most) internal system ambiguities][1606]
- [Change 'components' to 'bundles' where it makes sense semantically][1257]
- [add `Flags<T>` as a query to get flags of component][1172]
- [Rename `add_resource` to `insert_resource`][1356]
- [Update `init_resource` to not overwrite][1349]
- [Enable dynamic mutable access to component data][1284]
- [Get rid of `ChangedRes`][1313]
- [impl `SystemParam` for `Option<Res<T>>` / `Option<ResMut<T>>`][1494]
- [Add Window Resize Constraints][1409]
- [Add basic file drag and drop support][1096]
- [Modify Derive to allow unit structs for `RenderResources`.][1089]
- [bevy_render: load .spv assets][1104]
- [Expose wgpu backend in WgpuOptions and allow it to be configured from the environment][1042]
- [updates on diagnostics (log + new diagnostics)][1085]
- [enable change detection for labels][1155]
- [Name component with fast comparisons][1109]
- [Support for `!Send` tasks][1216]
- [Add missing `spawn_local` method to `Scope` in the single threaded executor case][1266]
- [Add bmp as a supported texture format][1081]
- [Add an alternative winit runner that can be started when not on the main thread][1063]
- [Added `use_dpi` setting to `WindowDescriptor`][1131]
- [Implement `Copy` for `ElementState`][1154]
- [Mutable mesh accessors: `indices_mut` and `attribute_mut`][1164]
- [Add support for OTF fonts][1200]
- [Add `from_xyz` to `Transform`][1212]
- [Adding `copy_texture_to_buffer` and `copy_texture_to_texture`][1236]
- [Added `set_minimized` and `set_position` to `Window`][1292]
- [Example for 2D Frustum Culling][1503]
- [Add remove resource to commands][1478]

### Changed

- [Bevy ECS V2][1525]
- [Fix Reflect serialization of tuple structs][1366]
- [color spaces and representation][1572]
- [Make vertex buffers optional][1485]
- [add to lower case to make asset loading case insensitive][1427]
- [Replace right/up/forward and counter parts with `local_x`/`local_y` and `local_z`][1476]
- [Use valid keys to initialize `AHasher` in `FixedState`][1268]
- [Change `Name` to take `Into<String>` instead of `String`][1283]
- [Update to wgpu-rs 0.7][542]
- [Update glam to 0.13.0.][1550]
- [use std clamp instead of Bevy's][1644]
- [Make `Reflect` impls unsafe (`Reflect::any` must return `self`)][1679]

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
[765]: https://github.com/bevyengine/bevy/pull/765
[772]: https://github.com/bevyengine/bevy/pull/772
[789]: https://github.com/bevyengine/bevy/pull/789
[791]: https://github.com/bevyengine/bevy/pull/791
[798]: https://github.com/bevyengine/bevy/pull/798
[801]: https://github.com/bevyengine/bevy/pull/801
[805]: https://github.com/bevyengine/bevy/pull/805
[808]: https://github.com/bevyengine/bevy/pull/808
[815]: https://github.com/bevyengine/bevy/pull/815
[820]: https://github.com/bevyengine/bevy/pull/820
[821]: https://github.com/bevyengine/bevy/pull/821
[829]: https://github.com/bevyengine/bevy/pull/829
[834]: https://github.com/bevyengine/bevy/pull/834
[836]: https://github.com/bevyengine/bevy/pull/836
[842]: https://github.com/bevyengine/bevy/pull/842
[843]: https://github.com/bevyengine/bevy/pull/843
[847]: https://github.com/bevyengine/bevy/pull/847
[852]: https://github.com/bevyengine/bevy/pull/852
[857]: https://github.com/bevyengine/bevy/pull/857
[859]: https://github.com/bevyengine/bevy/pull/859
[863]: https://github.com/bevyengine/bevy/pull/863
[864]: https://github.com/bevyengine/bevy/pull/864
[871]: https://github.com/bevyengine/bevy/pull/871
[876]: https://github.com/bevyengine/bevy/pull/876
[883]: https://github.com/bevyengine/bevy/pull/883
[887]: https://github.com/bevyengine/bevy/pull/887
[892]: https://github.com/bevyengine/bevy/pull/892
[893]: https://github.com/bevyengine/bevy/pull/893
[894]: https://github.com/bevyengine/bevy/pull/894
[895]: https://github.com/bevyengine/bevy/pull/895
[897]: https://github.com/bevyengine/bevy/pull/897
[903]: https://github.com/bevyengine/bevy/pull/903
[904]: https://github.com/bevyengine/bevy/pull/904
[905]: https://github.com/bevyengine/bevy/pull/905
[908]: https://github.com/bevyengine/bevy/pull/908
[914]: https://github.com/bevyengine/bevy/pull/914
[917]: https://github.com/bevyengine/bevy/pull/917
[920]: https://github.com/bevyengine/bevy/pull/920
[926]: https://github.com/bevyengine/bevy/pull/926
[928]: https://github.com/bevyengine/bevy/pull/928
[931]: https://github.com/bevyengine/bevy/pull/931
[932]: https://github.com/bevyengine/bevy/pull/932
[934]: https://github.com/bevyengine/bevy/pull/934
[937]: https://github.com/bevyengine/bevy/pull/937
[940]: https://github.com/bevyengine/bevy/pull/940
[945]: https://github.com/bevyengine/bevy/pull/945
[946]: https://github.com/bevyengine/bevy/pull/946
[947]: https://github.com/bevyengine/bevy/pull/947
[948]: https://github.com/bevyengine/bevy/pull/948
[952]: https://github.com/bevyengine/bevy/pull/952
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
[1035]: https://github.com/bevyengine/bevy/pull/1035
[1037]: https://github.com/bevyengine/bevy/pull/1037
[1038]: https://github.com/bevyengine/bevy/pull/1038
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
