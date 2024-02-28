<!-- MD024 - We want repeated headings in a changelog file -->
<!-- markdownlint-disable-file MD024 -->

# Changelog

While we try to keep the `Unreleased` changes updated, it is often behind and does not include
all merged pull requests. To see a list of all changes since the latest release, you may compare
current changes on git with [previous release tags][git_tag_comparison].

[git_tag_comparison]: https://github.com/bevyengine/bevy/compare/v0.13.0...main

## Version 0.13.0 (2024-02-17)

### A-Rendering + A-Windowing

- [Allow prepare_windows to run off main thread.][11660]
- [Allow prepare_windows to run off main thread on all platforms][11672]
- [don't run `create_surfaces` system if not needed][11720]
- [fix create_surfaces system ordering][11747]

### A-Animation + A-Reflection

- [Add type registrations for animation types][11889]

### A-Assets

- [Don't `.unwrap()` in `AssetPath::try_parse`][10452]
- [feat: `Debug` implemented for `AssetMode`][10494]
- [Remove rogue : from embedded_asset! docs][10516]
- [use `tree` syntax to explain bevy_rock file structure][10523]
- [Make AssetLoader/Saver Error type bounds compatible with anyhow::Error][10493]
- [Fix untyped labeled asset loading][10514]
- [Add `load_untyped` to LoadContext][10526]
- [fix example custom_asset_reader on wasm][10574]
- [`ReadAssetBytesError::Io`  exposes failing path][10450]
- [Added Method to Allow Pipelined Asset Loading][10565]
- [Add missing asset load error logs for load_folder and load_untyped][10578]
- [Fix wasm builds with file_watcher enabled][10589]
- [Do not panic when failing to create assets folder (#10613)][10614]
- [Use handles for queued scenes in SceneSpawner][10619]
- [Fix file_watcher feature hanging indefinitely][10585]
- [derive asset for enums][10410]
- [Ensure consistency between Un/Typed `AssetId` and `Handle`][10628]
- [Fix Asset Loading Bug][10698]
- [remove double-hasing of typeid for handle][10699]
- [AssetMetaMode][10623]
- [Fix GLTF scene dependencies and make full scene renders predictable][10745]
- [Print precise and correct watch warnings (and only when necessary)][10787]
- [Allow removing and reloading assets with live handles][10785]
- [Add GltfLoaderSettings][10804]
- [Refactor `process_handle_drop_internal()` in bevy_asset][10920]
- [fix base64 padding when loading a gltf file][11053]
- [assets should be kept on CPU by default][11212]
- [Don't auto create assets folder][11218]
- [Use `impl Into<A>` for `Assets::add`][10878]
- [Add `reserve_handle` to `Assets`.][10939]
- [Better error message on incorrect asset label][11254]
- [GLTF extension support][11138]
- [Fix embedded watcher to work with external crates][11370]
- [Added AssetLoadFailedEvent, UntypedAssetLoadFailedEvent][11369]
- [auto create imported asset folder if needed][11284]
- [Fix minor typo][11491]
- [Include asset path in get_meta_path panic message][11504]
- [Fix documentation for `AssetReader::is_directory` function][11538]
- [AssetSaver and AssetTransformer split][11260]
- [AssetPath source parse fix][11543]
- [Allow TextureAtlasBuilder in AssetLoader][11548]
- [Add a getter for asset watching status on `AssetServer`][11578]
- [Make SavedAsset::get_labeled accept &str as label][11612]
- [Added Support for Extension-less Assets][10153]
- [Fix embedded asset path manipulation][10383]
- [Fix AssetTransformer breaking LabeledAssets][11626]
- [Put asset_events behind a run condition][11800]
- [Use Asset Path Extension for `AssetLoader` Disambiguation][11644]

### A-Core + A-App

- [Add Accessibility plugin to default plugins docs][11512]

### A-Accessibility

- [Add html tags required for accessibility][10989]
- [missed negation during accessibility refactor][11206]

### A-Transform

- [Add `Transform::is_finite`][10592]

### A-ECS + A-Hierarchy

- [Add a doc note about despawn footgun][10889]

### A-Text

- [Rename `TextAlignment` to `JustifyText`.][10854]
- [Subtract 1 from text positions to account for glyph texture padding.][11662]

### A-Assets + A-UI

- [UI and unloaded assets: don't filter out nodes with an unloaded image][11205]

### A-Utils + A-Time

- [Make SystemTime available in both native and wasm][10980]

### A-Rendering + A-Assets

- [Fix shader import hot reloading on windows][10502]
- [Unload render assets from RAM][10520]
- [mipmap levels can be 0 and they should be interpreted as 1][11767]

### A-Physics

- [refactor collide code (Adopted)][11106]
- [Use `IntersectsVolume` for breakout example collisions][11500]

### A-ECS + A-Editor + A-App + A-Diagnostics

- [System Stepping implemented as Resource][8453]

### A-Reflection + A-Scenes

- [Implement and register Reflect (value) for CameraRenderGraph and CameraMainTextureUsages][11878]

### A-Audio + A-Windowing

- [Winit update: fix suspend on Android][11403]

### A-Build-System + A-Meta

- [Standardize toml format with taplo][10594]

### A-ECS + A-Time

- [Wait until `FixedUpdate` can see events before dropping them][10077]
- [Add First/Pre/Post/Last schedules to the Fixed timestep][10977]
- [Add run conditions for executing a system after a delay][11095]
- [Add paused run condition][11313]

### A-Meta

- [Add "update screenshots" to release checklist][10369]
- [Remove references to specific projects from the readme][10836]
- [Fix broken link between files][10962]
- [[doc] Fix typo in CONTRIBUTING.md][10971]
- [Remove unused namespace declarations][10965]
- [Add docs link to root `Cargo.toml`][10998]
- [Migrate third party plugins guidelines to the book][11242]
- [Run markdownlint][11386]
- [Improve `config_fast_builds.toml`][11529]
- [Use `-Z threads=0` option in `config_fast_builds.toml`][11541]
- [CONTRIBUTING.md: Mention splitting complex PRs][11703]

### A-Time

- [docs: use `read` instead of deprecated `iter`][10376]
- [Rename `Time::<Fixed>::overstep_percentage()` and `Time::<Fixed>::overstep_percentage_f64()`][10448]
- [Rename `Timer::{percent,percent_left}` to `Timer::{fraction,fraction_remaining}`][10442]
- [Document how to configure FixedUpdate][10564]
- [Add discard_overstep function to `Time<Fixed>`][10453]

### A-Assets + A-Reflection

- [Register `AssetPath` as type for reflection][11483]

### A-Diagnostics + A-Utils

- [move once from bevy_log to bevy_utils, to allow for it's use in bevy_ecs][11419]

### A-Windowing + A-App

- [Revert `App::run()` behavior/Remove `winit` specific code from `bevy_app`][10389]

### A-ECS + A-Scenes

- [Make the MapEntities trait generic over Mappers, and add a simpler EntityMapper][11428]

### A-Hierarchy

- [bevy_hierarchy: add some docs][10598]
- [Make bevy_app and reflect opt-out for bevy_hierarchy.][10721]
- [Add `bevy_hierarchy` Crate and plugin documentation][10951]
- [Rename "AddChild" to "PushChild"][11194]
- [Inline trivial methods in bevy_hierarchy][11332]

### A-ECS + A-App

- [Add custom schedule example][11527]

### A-Transform + A-Math

- [return Direction3d from Transform::up and friends][11604]

### A-UI + A-Text

- [Improved Text Rendering][10537]
- [Feature-gate all references to `bevy_text` in `bevy_ui`][11391]

### A-Input

- [Make ButtonSettings.is_pressed/released public][10534]
- [Rename `Input` to `ButtonInput`][10859]
- [Add method to check if all inputs are pressed][11010]
- [Add window entity to TouchInput events][11128]
- [Extend `Touches` with clear and reset methods][10930]
- [Add logical key data to KeyboardInput][11400]
- [Derive Ord for GamepadButtonType.][11791]
- [Add delta to CursorMoved event][11710]

### A-Rendering + A-Diagnostics

- [Use `warn_once` where relevant instead of manually implementing a single warn check][11693]

### A-Rendering

- [Fix bevy_pbr shader function name][10423]
- [Implement Clone for VisibilityBundle and SpatialBundle][10394]
- [Reexport `wgpu::Maintain`][10461]
- [Use a consistent scale factor and resolution in stress tests][10474]
- [Ignore inactive cameras][10543]
- [Add shader_material_2d example][10542]
- [More inactive camera checks][10555]
- [Fix post processing example to only run effect on camera with settings component][10560]
- [Make sure added image assets are checked in camera_system][10556]
- [Ensure ExtendedMaterial works with reflection (to enable bevy_egui_inspector integration)][10548]
- [Explicit color conversion methods][10321]
- [Re-export wgpu BufferAsyncError][10611]
- [Improve shader_material example][10547]
- [Non uniform transmission samples][10674]
- [Explain how `AmbientLight` is inserted and configured][10712]
- [Add wgpu_pass method to TrackedRenderPass][10722]
- [Add a `depth_bias` to `Material2d`][10683]
- [Use as_image_copy where possible][10733]
- [impl `From<Color>` for ClearColorConfig][10734]
- [Ensure instance_index push constant is always used in prepass.wgsl][10706]
- [Bind group layout entries][10224]
- [prepass vertex shader always outputs world position][10657]
- [Swap material and mesh bind groups][10485]
- [try_insert Aabbs][10801]
- [Fix prepass binding issues causing crashes when not all prepass bindings are used][10788]
- [Fix binding group in custom_material_2d.wgsl][10841]
- [Normalize only nonzero normals for mikktspace normal maps][10905]
- [light renderlayers][10742]
- [Explain how RegularPolygon mesh is generated][10927]
- [Fix Mesh2d normals on webgl][10967]
- [Update to wgpu 0.18][10266]
- [Fix typo in docs for `ViewVisibility`][10979]
- [Add docs to bevy_sprite a little][10947]
- [Fix BindingType import warning][10818]
- [Update texture_atlas example with different padding and sampling][10073]
- [Update AABB when Sprite component changes in calculate_bounds_2d()][11016]
- [OrthographicProjection.scaling_mode is not just for resize][11024]
- [Derive `Debug` for `BloomCompositeMode`][11041]
- [Document None conditions on compute_aabb][11051]
- [Replace calculation with function call][11077]
- [Register Camera types.][11069]
- [Add example for pixel-perfect grid snapping in 2D][8112]
- [Misc cleanup][11134]
- [Keep track of when a texture is first cleared][10325]
- [Fix Mesh::ATTRIBUTE_UV_0 documentation][11110]
- [Do not load prepass normals for transmissive materials][11140]
- [Export tonemapping_pipeline_key (2d), alpha_mode_pipeline_key][11166]
- [Simplify examples/3d/orthographic][11045]
- [Implement lightmaps.][10231]
- [Bump the vertex attribute index for prepass joints.][11191]
- [Fix: Gizmos crash due to the persistence policy being set to `Unload`. Change it to `Keep`][11192]
- [Usability methods for RenderTargets and image handles][10736]
- [Explain Camera physical size is in pixel][11189]
- [update Outdated comment][11243]
- [Revert "Implement minimal reflection probes. (#10057)"][11307]
- [Explain OrthographicProjection.scale][11023]
- [`Mul<f32>` for ScalingMode][11030]
- [Rustdoc examples for OrthographicProjection][11031]
- [Option to enable deterministic rendering][11248]
- [Fix ssao only sampling mip 0][11292]
- [Revert "Implement minimal reflection probes. (#10057)"][11307]
- [Sprite slicing and tiling][10588]
- [Approximate indirect specular occlusion][11152]
- [Texture Atlas rework][5103]
- [Exposure settings (adopted)][11347]
- [Remove Vec from GpuArrayBuffer][11368]
- [Make `DynamicUniformBuffer::push` accept an `&T` instead of `T`][11373]
- [Restore brightness in the remaining three examples after exposure PR][11389]
- [Customizable camera main texture usage][11412]
- [Cleanup deterministic example][11416]
- [Implement minimal reflection probes (fixed macOS, iOS, and Android).][11366]
- [optimize  batch_and_prepare_render_phase][11323]
- [add `storage_texture` option to as_bind_group macro][9943]
- [Revert rendering-related associated type name changes][11027]
- [Meshlet prep][11442]
- [Reuse sampler when creating cached bind groups][10610]
- [Add Animated Material example][11524]
- [Update to wgpu 0.19 and raw-window-handle 0.6][11280]
- [Fix bug where Sprite::rect was ignored][11480]
- [Added documentation explaining the difference between lumens and luxes][11551]
- [Fix infinite asset preparation due to undrained AssetEvent events][11383]
- [Workaround for ICE in the DXC shader compiler in debug builds with an `EnvironmentMapLight`][11487]
- [Refactor tonemapping example's image viewer update into two systems][11519]
- [Add `Mesh` transformation][11454]
- [Fix specular envmap in deferred][11534]
- [Add `Meshable` trait and implement meshing for 2D primitives][11431]
- [Optimize extract_clusters and prepare_clusters systems][10633]
- [RenderAssetPersistencePolicy → RenderAssetUsages][11399]
- [RenderGraph Labelization][10644]
- [Gate diffuse and specular transmission behind shader defs][11627]
- [Add helpers for translate, rotate, and scale operations - Mesh][11675]
- [CameraProjection::compute_frustum][11139]
- [Added formats to `MeshVertexAttribute` constant's docstrings][11705]
- [Async pipeline compilation][10812]
- [sort by pipeline then mesh for non transparent passes for massively better batching][11671]
- [Added remove_indices to Mesh][11733]
- [Implement irradiance volumes.][10268]
- [Mesh insert indices][11745]
- [Don't try to create a uniform buffer for light probes if there are no views.][11751]
- [Properly check for result when getting pipeline in Msaa][11758]
- [wait for render app when main world is dropped][11737]
- [Deprecate shapes in `bevy_render::mesh::shape`][11773]
- [Cache the QueryState used to drop swapchain TextureViews][11781]
- [Multithreaded render command encoding][9172]
- [Fix `Quad` deprecation message mentioning a type that doesn't exist][11798]
- [Stop extracting mesh entities to the render world.][11803]
- [Stop copying the light probe array to the stack in the shader.][11805]
- [Add `Mesh::merge`][11456]
- [Call a TextureAtlasLayout a layout and not an atlas][11783]
- [fix shadow batching][11645]
- [Change light defaults & fix light examples][11581]
- [New Exposure and Lighting Defaults (and calibrate examples)][11868]
- [Change MeshUniform::new() to be public.][11880]
- [Rename Core Render Graph Labels][11882]
- [Support optional clear color in ColorAttachment.][11884]
- [irradiance: use textureSampleLevel for WebGPU support][11893]
- [Add configuration for async pipeline creation on RenderPlugin][11847]
- [Derive Reflect for Exposure][11907]
- [Add `MeshPipelineKey::LIGHTMAPPED` as applicable during the shadow map pass.][11910]
- [Irradiance volume example tweaks][11911]
- [Disable irradiance volumes on WebGL and WebGPU.][11909]
- [Remove `naga_oil` dependency from `bevy_pbr`][11914]

### A-Scenes

- [Re-export `ron` in `bevy_scene`][10529]
- [Fix load scene example to use proper serialization format for rotation field][10638]
- [Mention DynamicSceneBuilder in doc comment][10780]
- [Mention DynamicSceneBuilder in scene example][10441]
- [Implement Std traits for `SceneInstanceReady`][11003]
- [Change SceneSpawner::spawn_dynamic_sync to return InstanceID][11239]
- [Fix scene example][11289]
- [Send `SceneInstanceReady` only once per scene][11002]

### A-Utils

- [bevy_utils: Export `generate_composite_uuid` utility function][10496]
- [Save an instruction in `EntityHasher`][10648]
- [Add SystemTime to bevy_utils][11054]
- [Re-export smallvec crate from bevy_utils][11006]
- [Enable cloning EntityHashMap and PreHashMap][11178]
- [impl `Borrow` and `AsRef` for `CowArc`][11616]
- [Hash stability guarantees][11690]
- [Deprecating hashbrown reexports][11721]
- [Update ahash to 0.8.7][11785]

### A-UI

- [ui material: fix right border width][10421]
- [Add PartialEq to Anchor][10424]
- [UI Material: each material should have its own buffer][10422]
- [UI Materials: ignore entities with a `BackgroundColor` component][10434]
- [Fix panic when using image in UiMaterial][10591]
- [Make clipped areas of UI nodes non-interactive][10454]
- [Fix typo in resolve_outlines_system][10730]
- [Clip outlines by the node's own clipping rect, not the parent's.][10922]
- [Give UI nodes with `Display::None` an empty clipping rect][10942]
- [Create serialize feature for bevy_ui][11188]
- [Made the remaining types from bevy_ui to reflect the Default trait if…][11199]
- [Camera-driven UI][10559]
- [fix occasional crash moving ui root nodes][11371]
- [Fix panic on Text UI without Cameras][11405]
- [Allow user to choose default ui camera][11436]
- [Rustdoc links in bevy_ui][11555]
- [Avoid unconditionally unwrapping the Result - UI Stack System][11575]

### A-Assets + A-Diagnostics

- [Fix asset loader registration warning][11870]

### A-Audio + A-Reflection

- [Reflect and register audio-related types][10484]

### A-Audio

- [Add `VolumeLevel::ZERO`][10608]
- [Deduplicate systems in bevy_audio][10906]
- [Non-Intrusive refactor of `play_queued_audio_system()`][10910]
- [docs: AnimationPlayer::play doesn't have transition_duration arg][10970]
- [Remove the ability to ignore global volume][11092]
- [Optional override for global spatial scale][10419]

### A-Tasks

- [Make FakeTask public on singlethreaded context][10517]
- [Re-export `futures_lite` in `bevy_tasks`][10670]
- [bump bevy_tasks futures-lite to 2.0.1][10675]
- [Fix wrong transmuted type in `TaskPool::scope_with_executor_inner`][11455]
- [Use `std::thread::sleep` instead of spin-waiting in the async_compute example][11856]

### A-ECS

- [Use `EntityHashMap` for `EntityMapper`][10415]
- [Allow registering boxed systems][10378]
- [Remove unnecessary if statement in scheduler][10446]
- [Optimize `Entity::eq`][10519]
- [Add 'World::run_system_with_input' function + allow `World::run_system` to get system output][10380]
- [Update `Event` send methods to return `EventId`][10551]
- [Some docs for IntoSystemSet][10563]
- [Link to `In` in `pipe` documentation][10596]
- [Optimise `Entity` with repr align & manual `PartialOrd`/`Ord`][10558]
- [Allow #[derive(Bundle)] on tuple structs (take 3)][10561]
- [Add an `Entry` api to `EntityWorldMut`.][10650]
- [Make impl block for RemovedSystem generic][10651]
- [Append commands][10400]
- [Rustdoc example for Ref][10682]
- [Link to `Main` schedule docs from other schedules][10691]
- [Warn that Added/Changed filters do not see deferred changes][10681]
- [Fix non-functional nondeterministic_system_order example][10719]
- [Copy over docs for `Condition` trait from PR #10718][10748]
- [Implement `Drop` for `CommandQueue`][10746]
- [Split WorldQuery into WorldQueryData and WorldQueryFilter][9918]
- [Make IntoSystemConfigs::into_configs public API (visible in docs)][10624]
- [Override QueryIter::fold to port Query::for_each perf gains to select Iterator combinators][6773]
- [Deprecate QueryState::for_each_unchecked][10815]
- [Clarifying Commands' purpose][10837]
- [Make ComponentId typed in Components][10770]
- [Reduced `TableRow` `as` Casting][10811]
- [Add `EntityCommands.retain` and `EntityWorldMut.retain`][10873]
- [Remove unnecessary ResMut in examples][10879]
- [Add a couple assertions for system types][10893]
- [Remove reference to default schedule][10918]
- [Improve `EntityWorldMut.remove`, `retain` and `despawn` docs by linking to more detail][10943]
- [Reorder fields in SystemSchedule][10764]
- [Rename `WorldQueryData` & `WorldQueryFilter` to `QueryData` & `QueryFilter`][10779]
- [Fix soundness of `UnsafeWorldCell` usage example][10941]
- [Actually check alignment in BlobVec test aligned_zst][10885]
- [Rename `Q` type parameter to `D` when referring to `WorldQueryData`][10782]
- [Allow the editing of startup schedules][10969]
- [Auto insert sync points][9822]
- [Simplify lifetimes in `QueryState` methods][10937]
- [Add is_resource_changed_by_id + is_resource_added_by_id][11012]
- [Rename some lifetimes (ResMut etc) for clarity][11021]
- [Add non-existent entity behavior to Has doc][11025]
- [Fix typo in docs for Has][11028]
- [Add insert_state to App.][11043]
- [Explain Changed, Added are not archetype filters][11049]
- [Add missing colon in `States` documentation][11064]
- [Explain EventWriter limits concurrency][11063]
- [Better doc for SystemName][11084]
- [impl ExclusiveSystemParam for WorldId][11164]
- [impl ExclusiveSystemParam for PhantomData][11153]
- [Remove little warn on bevy_ecs][11149]
- [Rename `ArchetypeEntity::entity` into `ArchetypeEntity::id`][11118]
- [Fixed Typo in the description of EntityMut][11103]
- [Implement Deref and DerefMut for In][11104]
- [impl ExclusiveSystemParam for SystemName][11163]
- [Print a warning for un-applied commands being dropped from a CommandQueue][11146]
- [Implement TypePath for EntityHash][11195]
- [Fix integer overflow in BlobVec::push for ZST][10799]
- [Fix integer overflow in BlobVec::reserve_exact][11234]
- [StateTransitionEvent][11089]
- [Restore support for running `fn` `EntityCommands` on entities that might be despawned][11107]
- [Remove apply_deferred example][11142]
- [Minimize small allocations by dropping the tick Vecs from Resources][11226]
- [Change Entity::generation from u32 to NonZeroU32 for niche optimization][9907]
- [fix B0003 example and update logs][11162]
- [Unified identifer for entities & relations][9797]
- [Simplify conditions][11316]
- [Add example using `State` in docs][11319]
- [Skip rehashing TypeIds][11268]
- [Make `TypeId::hash` more robust in case of upstream rustc changes][11334]
- [Fix doc of [`Schedules`] to mention exclusion of current schedule.][11360]
- [Dynamic queries and builder API][9774]
- [Remove duplicate `#[automatically_derived]` in ECS macro][11388]
- [Get Change Tick methods for Resources][11404]
- [Optional state][11417]
- [Double the capacity when BlobVec is full][11167]
- [document which lifetime is needed for systemparam derive][11321]
- [refactor: Simplify lifetimes for `Commands` and related types][11445]
- [Implement `Debug` for `CommandQueue`][11444]
- [Fix typo in comment][11486]
- [Rename Schedule::name to Schedule::label][11531]
- [Exclusive systems can now be used for one-shot systems][11560]
- [added ability to get `Res<T>` from `World` with `World::get_resource_ref`][11561]
- [bevy_ecs: Add doc example for par_iter_mut (#11311)][11499]
- [Add an example demonstrating how to send and receive events in the same system][11574]
- [Add a doctest example for EntityMapper][11583]
- [Rephrase comment about `Local<T>` for clarity. (Adopted)][11129]
- [Use batch spawn in benchmarks][11611]
- [Fix bug where events are not being dropped][11528]
- [Make Archetypes.archetype_component_count private][10774]
- [Deprecated Various Component Methods from `Query` and `QueryState`][9920]
- [`System::type_id` Consistency][11728]
- [Typo in [`ScheduleLabel`] derive macro][11764]
- [Mention Resource where missing from component/resource related type docs][11769]
- [Expose query accesses][11700]
- [Add a method for detecting changes within a certain scope][11687]
- [Fix double indirection when applying command queues][11822]
- [Immediately poll the executor once before spawning it as a task][11801]
- [Fix small docs misformat in `BundleInfo::new`][11855]
- [`FilteredEntityRef` conversions][11838]

### A-Rendering + A-Animation

- [TextureAtlasBuilder now respects insertion order][11474]
- [normalize joint weights][10539]

### A-ECS + A-Meta

- [resolve all internal ambiguities][10411]

### A-Rendering + A-UI

- [Provide GlobalsUniform in UiMaterial shaders][10739]
- [Include UI node size in the vertex inputs for UiMaterial.][11722]
- [UI Texture 9 slice][11600]
- [Optional ImageScaleMode][11780]

### A-Math

- [Define a basic set of Primitives][10466]
- [Add and impl Primitives][10580]
- [Add winding order for `Triangle2d`][10620]
- [Use minor and major radii for `Torus` primitive shape][10643]
- [Remove `From` implementations from the direction types][10857]
- [Impl `TryFrom` vector for directions and add `InvalidDirectionError`][10884]
- [Add `Direction2d::from_xy` and `Direction3d::from_xyz`][10882]
- [Implement `Neg` for `Direction2d` and `Direction3d`][11179]
- [Add constants for `Direction2d` and `Direction3d`][11180]
- [Add `approx` feature to `bevy_math`][11176]
- [Add `libm` feature to `bevy_math`][11238]
- [Add `new_and_length` method to `Direction2d` and `Direction3d`][11172]
- [Update `glam`, `encase` and `hexasphere`][11082]
- [Implement bounding volume types][10946]
- [Remove `Default` impl for `CubicCurve`][11335]
- [Implement bounding volumes for primitive shapes][11336]
- [Improve `Rectangle` and `Cuboid` consistency][11434]
- [Change `Ellipse` representation and improve helpers][11435]
- [Add `Aabb2d::new` and `Aabb3d::new` constructors][11433]
- [Add geometric primitives to `bevy_math::prelude`][11432]
- [Direction: Rename `from_normalized` to `new_unchecked`][11425]
- [Implement bounding volume intersections][11439]
- [Add `new` constructors for `Circle` and `Sphere`][11526]
- [Derive PartialEq, Serialize, Deserialize and Reflect on primitives][11514]
- [Document RegularPolygon][11017]
- [Add RayTest2d and RayTest3d][11310]
- [Add more constructors and math helpers for primitive shapes][10632]
- [Add `Capsule2d` primitive][11585]
- [Add volume cast intersection tests][11586]
- [Add Clone to intersection test types][11640]
- [Implement `approx` traits for direction types][11650]
- [Support rotating `Direction3d` by `Quat`][11649]
- [Rename RayTest to RayCast][11635]
- [Add example for bounding volumes and intersection tests][11666]
- [Dedicated primitive example][11697]
- [Un-hardcode positions and colors in `2d_shapes` example][11867]

### A-Build-System

- [check for all-features with cargo-deny][10544]
- [Bump actions/github-script from 6 to 7][10653]
- [Add doc_markdown clippy linting config to cargo workspace][10640]
- [Enable `clippy::undocumented_unsafe_blocks` warning across the workspace][10646]
- [Remove trailing whitespace][10723]
- [Move remaining clippy lint definitions to Cargo.toml][10672]
- [Add `clippy::manual_let_else` at warn level to lints][10684]
- [Remove unused import][10963]
- [Rename functions and variables to follow code style][10961]
- [Remove unused variable][10966]
- [add libxkbcommon-x11-0 to the default linux dependencies][11060]
- [fix patches for example showcase after winit update][11058]
- [finish cleaning up dependency bans job][11059]
- [Bump actions/upload-artifact from 2 to 4][11014]
- [Publish dev-docs with Github Pages artifacts (2nd attempt)][10892]
- [example showcase patches: use default instead of game mode for desktop][11250]
- [Bump toml_edit in build-template-pages tool][11342]
- [Miri is failing on latest nightly: pin nightly to last known working version][11421]
- [Bump dev-docs pages actions][11418]
- [Unpin nightly for miri][11462]
- [documentation in CI: remove lock file][11507]
- [Bump actions/cache from 3 to 4][11469]
- [simplify animated_material example][11576]
- [example showcase: fix window resized patch][11596]
- [run examples on macOS to validate PRs][11630]
- [Inverse `missing_docs` logic][11676]
- [Bump peter-evans/create-pull-request from 5 to 6][11712]

### A-Gizmos

- [Fix float precision issue in the gizmo shader][10408]
- [Gizmo Arrows][10550]
- [Move Circle Gizmos to Their Own File][10631]
- [move gizmo arcs to their own file][10660]
- [Warn when bevy_sprite and bevy_pbr are not enabled with bevy_gizmos][11296]
- [Multiple Configurations for Gizmos][10342]
- [Fix gizmos app new panic][11420]
- [Use Direction3d for gizmos.circle normal][11422]
- [Implement Arc3D for Gizmos][11540]
- [Insert Gizmos config instead of Init][11580]
- [Drawing Primitives with Gizmos][11072]
- [fix(primitives): fix polygon gizmo rendering bug][11699]
- [Fix global wireframe behavior not being applied on new meshes][11792]
- [Overwrite gizmo group in `insert_gizmo_group`][11860]

### A-Rendering + A-Math

- [Split `Ray` into `Ray2d` and `Ray3d` and simplify plane construction][10856]
- [Introduce AspectRatio struct][10368]
- [Implement meshing for `Capsule2d`][11639]
- [Implement `Meshable` for some 3D primitives][11688]

### A-Core

- [Derive `Debug` for `Framecount`][11573]
- [Don't unconditionally enable bevy_render or bevy_assets if mutli-threaded feature is enabled][11726]

### A-Windowing

- [Some explanations for Window component][10714]
- [don't run update before window creation in winit][10741]
- [add new event `WindowOccluded` from winit][10735]
- [Add comment about scale factor in `WindowMode`][10872]
- [Refactor function `update_accessibility_nodes`][10911]
- [Change `Window` scale factor to f32 (adopted)][10897]
- [Reexport winit::platform::android::activity::* in bevy_winit][11011]
- [Update winit dependency to 0.29][10702]
- [Remove CanvasParentResizePlugin][11057]
- [Use `WindowBuilder::with_append()` to append canvas][11065]
- [Fix perf degradation on web builds][11227]
- [mobile and webgpu: trigger redraw request when needed and improve window creation][11245]
- [Remove unnecessary unsafe impls for WinitWindows on Wasm][11270]
- [Fix Reactive and ReactiveLowPower update modes][11325]
- [Change `WinitPlugin` defaults to limit game update rate when window is not visible (for real this time)][11305]
- [Cleanup bevy winit][11489]
- [Add `name` to `bevy::window::Window`][7650]
- [Avoid unwraps in winit fullscreen handling code][11735]

### A-UI + A-Transform + A-Text

- [UI text rotation and scaling fix][11326]

### A-Animation

- [Fix animations resetting after repeat count][10540]
- [Add Debug, PartialEq and Eq derives to bevy_animation.][10562]
- [support all types of animation interpolation from gltf][10755]
- [Clean up code to find the current keyframe][11306]
- [Skip alloc when updating animation path cache][11330]
- [Replace the `cubic_spline_interpolation` macro with a generic function][11605]
- [Animatable trait for interpolation and blending][4482]

### A-ECS + A-Pointers

- [Replace pointer castings (`as`) by their API equivalent][11818]

### A-ECS + A-Utils

- [Add helper macro's for logging only once][10808]
- [Move `EntityHash` related types into `bevy_ecs`][11498]

### A-Reflection

- [Fix issue with `Option` serialization][10705]
- [fix `insert_reflect` panic caused by `clone_value`][10627]
- [Remove pointless trait implementation exports in `bevy_reflect`][10771]
- [Fix nested generics in Reflect derive][10791]
- [Fix debug printing for dynamic types][10740]
- [reflect: maximally relax `TypePath` bounds][11037]
- [Use `static_assertions` to check for trait impls][11407]
- [Add `ReflectFromWorld` and replace the `FromWorld` requirement on `ReflectComponent` and `ReflectBundle` with `FromReflect`][9623]
- [Fix reflected serialization/deserialization on `Name` component][11447]
- [Add Reflection for Wrapping/Saturating types][11397]
- [Remove TypeUuid][11497]
- [Fix warnings in bevy_reflect][11556]
- [bevy_reflect: Type parameter bounds][9046]
- [bevy_reflect: Split `#[reflect(where)]`][11597]
- [reflection: replace `impl_reflect_struct` with `impl_reflect`][11437]
- [Add the ability to manually create ParsedPaths (+ cleanup)][11029]
- [bevy_reflect: Reflect `&'static str`][11686]
- [Improve DynamicStruct::insert][11068]
- [Missing registrations][11736]
- [Add `ReflectKind`][11664]
- [doc(bevy_reflect): add note about trait bounds on `impl_type_path`][11810]
- [bevy_reflect_derive: Clean up attribute logic][11777]

### A-ECS + A-Tasks

- [Async channel v2][10692]

### A-Pointers

- [Remove a ptr-to-int cast in `CommandQueue::apply`][10475]
- [Fix memory leak in dynamic ECS example][11461]
- [bevy_ptr: fix `unsafe_op_in_unsafe_fn` lint][11610]

### A-ECS + A-Reflection

- [Adding derive Reflect for tick structs][11641]

### A-Reflection + A-Gizmos

- [`#[derive(Reflect)]` on `GizmoConfig`][10483]
- [Register `WireframeColor`][10486]

### No area label

- [Fix intra-doc link warnings][10445]
- [Fix minor issues with custom_asset example][10337]
- [Prepend `root_path` to meta path in HttpWasmAssetReader][10527]
- [support required features in wasm examples showcase][10577]
- [examples showcase: use patches instead of sed for wasm hacks][10601]
- [Add [lints] table, fix  adding `#![allow(clippy::type_complexity)]` everywhere][10011]
- [Bumps async crates requirements to latest major version][10370]
- [delete methods deprecated in 0.12][10693]
- [Ran `cargo fmt` on `benches` crate][10758]
- [Remove unnecessary path prefixes][10749]
- [Fix typos in safety comment][10827]
- [Substitute `get(0)` with `first()`][10847]
- [Remove identity `map` calls][10848]
- [Renamed Accessibility plugin to AccessKitPlugin in bevy_winit][10914]
- [Reorder impl to be the same as the trait][11076]
- [Replace deprecated elements][10999]
- [Remove unnecessary parentheses][10990]
- [Replace deprecated elements][10999]
- [Simplify equality assertions][10988]
- [Add Solus package requrements to linux_dependencies.md][10996]
- [Update base64 requirement from 0.13.0 to 0.21.5][10336]
- [Update sysinfo version to 0.30.0][11071]
- [Remove unnecessary parens][11075]
- [Reorder impl to be the same as the trait][11076]
- [Fix ci xvfb][11143]
- [Replace or document ignored doctests][11040]
- [Add static assertions to bevy_utils for compile-time checks][11182]
- [Fix missed explicit conversions in examples][11261]
- [Remove unused event-listener dependency][11269]
- [Fixed typo in generate_custom_mesh.rs example][11293]
- [Extract examples `CameraController` into a module][11338]
- [Use EntityHashMap whenever possible][11353]
- [Fix link to plugin guidelines][11379]
- [[doc] Fix typo and formatting in CONTRIBUTING.md][11381]
- [add a required feature for shader_material_glsl][11440]
- [Update ruzstd requirement from 0.4.0 to 0.5.0][11467]
- [Tweak gamepad viewer example style][11484]
- [Add `.toml` extension to `.cargo/config_fast_builds`][11506]
- [Add README to benches][11508]
- [Fix panic in examples using argh on the web][11513]
- [Fix cyclic dep][11523]
- [Enable the `unsafe_op_in_unsafe_fn` lint][11591]
- [Update erased-serde requirement from 0.3 to 0.4][11599]
- [Fix example send_and_receive_events][11615]
- [Update cursor.rs][11617]
- [Use the `Continuous` update mode in stress tests when unfocused][11652]
- [Don't auto insert on the extract schedule][11669]
- [Update tracing-tracy requirement from 0.10.4 to 0.11.0 and tracy-client requirement from 0.16.4 to 0.17.0][11678]
- [Use TypeIdMap whenever possible][11684]
- [Fix a few typos in error docs][11709]
- [bevy_render: use the non-send marker from bevy_core][11725]
- [Ignore screenshots generated by `screenshot` example][11797]
- [Docs reflect that `RemovalDetection` also yields despawned entities][11795]
- [bevy_dynamic_plugin: fix `unsafe_op_in_unsafe_fn` lint][11622]
- [Replace `crossbeam::scope` reference with `thread::scope` in docs][11832]
- [Use question mark operator when possible][11865]
- [Fix a few Clippy lints][11866]
- [WebGPU: fix web-sys version][11894]
- [Remove map_flatten from linting rules][11913]
- [Fix duplicate `encase_derive_impl` dependency][11915]

### A-App

- [add regression test for #10385/#10389][10609]
- [Fix typos plugin.rs][11193]
- [Expressively define plugins using functions][11080]
- [Mark `DynamicPluginLoadError` internal error types as source][11618]

### A-Diagnostics

- [Fix Line for tracy version][10663]
- [Some doc to bevy_diagnostic][11020]
- [Print to stderr from panic handler in LogPlugin][11170]
- [Add ability to panic to logs example][11171]
- [Make sure tracy deps conform to compatibility table][11331]
- [Describe purpose of bevy_diagnostic][11327]
- [Add support for updating the tracing subscriber in LogPlugin][10822]
- [Replace `DiagnosticId` by `DiagnosticPath`][9266]
- [fix link to tracy][11521]
- [Fix sysinfo CPU brand output][11850]

### A-Rendering + A-ECS

- [Explain where rendering is][11018]

### A-Assets + A-Math

- [Use glam for computing gLTF node transform][11361]

[4482]: https://github.com/bevyengine/bevy/pull/4482
[5103]: https://github.com/bevyengine/bevy/pull/5103
[6773]: https://github.com/bevyengine/bevy/pull/6773
[7650]: https://github.com/bevyengine/bevy/pull/7650
[8112]: https://github.com/bevyengine/bevy/pull/8112
[8453]: https://github.com/bevyengine/bevy/pull/8453
[9046]: https://github.com/bevyengine/bevy/pull/9046
[9172]: https://github.com/bevyengine/bevy/pull/9172
[9266]: https://github.com/bevyengine/bevy/pull/9266
[9623]: https://github.com/bevyengine/bevy/pull/9623
[9774]: https://github.com/bevyengine/bevy/pull/9774
[9797]: https://github.com/bevyengine/bevy/pull/9797
[9822]: https://github.com/bevyengine/bevy/pull/9822
[9907]: https://github.com/bevyengine/bevy/pull/9907
[9918]: https://github.com/bevyengine/bevy/pull/9918
[9920]: https://github.com/bevyengine/bevy/pull/9920
[9943]: https://github.com/bevyengine/bevy/pull/9943
[10011]: https://github.com/bevyengine/bevy/pull/10011
[10073]: https://github.com/bevyengine/bevy/pull/10073
[10077]: https://github.com/bevyengine/bevy/pull/10077
[10153]: https://github.com/bevyengine/bevy/pull/10153
[10224]: https://github.com/bevyengine/bevy/pull/10224
[10231]: https://github.com/bevyengine/bevy/pull/10231
[10266]: https://github.com/bevyengine/bevy/pull/10266
[10268]: https://github.com/bevyengine/bevy/pull/10268
[10321]: https://github.com/bevyengine/bevy/pull/10321
[10325]: https://github.com/bevyengine/bevy/pull/10325
[10336]: https://github.com/bevyengine/bevy/pull/10336
[10337]: https://github.com/bevyengine/bevy/pull/10337
[10342]: https://github.com/bevyengine/bevy/pull/10342
[10368]: https://github.com/bevyengine/bevy/pull/10368
[10369]: https://github.com/bevyengine/bevy/pull/10369
[10370]: https://github.com/bevyengine/bevy/pull/10370
[10376]: https://github.com/bevyengine/bevy/pull/10376
[10378]: https://github.com/bevyengine/bevy/pull/10378
[10380]: https://github.com/bevyengine/bevy/pull/10380
[10383]: https://github.com/bevyengine/bevy/pull/10383
[10389]: https://github.com/bevyengine/bevy/pull/10389
[10394]: https://github.com/bevyengine/bevy/pull/10394
[10400]: https://github.com/bevyengine/bevy/pull/10400
[10408]: https://github.com/bevyengine/bevy/pull/10408
[10410]: https://github.com/bevyengine/bevy/pull/10410
[10411]: https://github.com/bevyengine/bevy/pull/10411
[10415]: https://github.com/bevyengine/bevy/pull/10415
[10419]: https://github.com/bevyengine/bevy/pull/10419
[10421]: https://github.com/bevyengine/bevy/pull/10421
[10422]: https://github.com/bevyengine/bevy/pull/10422
[10423]: https://github.com/bevyengine/bevy/pull/10423
[10424]: https://github.com/bevyengine/bevy/pull/10424
[10434]: https://github.com/bevyengine/bevy/pull/10434
[10441]: https://github.com/bevyengine/bevy/pull/10441
[10442]: https://github.com/bevyengine/bevy/pull/10442
[10445]: https://github.com/bevyengine/bevy/pull/10445
[10446]: https://github.com/bevyengine/bevy/pull/10446
[10448]: https://github.com/bevyengine/bevy/pull/10448
[10450]: https://github.com/bevyengine/bevy/pull/10450
[10452]: https://github.com/bevyengine/bevy/pull/10452
[10453]: https://github.com/bevyengine/bevy/pull/10453
[10454]: https://github.com/bevyengine/bevy/pull/10454
[10461]: https://github.com/bevyengine/bevy/pull/10461
[10466]: https://github.com/bevyengine/bevy/pull/10466
[10474]: https://github.com/bevyengine/bevy/pull/10474
[10475]: https://github.com/bevyengine/bevy/pull/10475
[10483]: https://github.com/bevyengine/bevy/pull/10483
[10484]: https://github.com/bevyengine/bevy/pull/10484
[10485]: https://github.com/bevyengine/bevy/pull/10485
[10486]: https://github.com/bevyengine/bevy/pull/10486
[10493]: https://github.com/bevyengine/bevy/pull/10493
[10494]: https://github.com/bevyengine/bevy/pull/10494
[10496]: https://github.com/bevyengine/bevy/pull/10496
[10502]: https://github.com/bevyengine/bevy/pull/10502
[10514]: https://github.com/bevyengine/bevy/pull/10514
[10516]: https://github.com/bevyengine/bevy/pull/10516
[10517]: https://github.com/bevyengine/bevy/pull/10517
[10519]: https://github.com/bevyengine/bevy/pull/10519
[10520]: https://github.com/bevyengine/bevy/pull/10520
[10523]: https://github.com/bevyengine/bevy/pull/10523
[10526]: https://github.com/bevyengine/bevy/pull/10526
[10527]: https://github.com/bevyengine/bevy/pull/10527
[10529]: https://github.com/bevyengine/bevy/pull/10529
[10534]: https://github.com/bevyengine/bevy/pull/10534
[10537]: https://github.com/bevyengine/bevy/pull/10537
[10539]: https://github.com/bevyengine/bevy/pull/10539
[10540]: https://github.com/bevyengine/bevy/pull/10540
[10542]: https://github.com/bevyengine/bevy/pull/10542
[10543]: https://github.com/bevyengine/bevy/pull/10543
[10544]: https://github.com/bevyengine/bevy/pull/10544
[10547]: https://github.com/bevyengine/bevy/pull/10547
[10548]: https://github.com/bevyengine/bevy/pull/10548
[10550]: https://github.com/bevyengine/bevy/pull/10550
[10551]: https://github.com/bevyengine/bevy/pull/10551
[10555]: https://github.com/bevyengine/bevy/pull/10555
[10556]: https://github.com/bevyengine/bevy/pull/10556
[10558]: https://github.com/bevyengine/bevy/pull/10558
[10559]: https://github.com/bevyengine/bevy/pull/10559
[10560]: https://github.com/bevyengine/bevy/pull/10560
[10561]: https://github.com/bevyengine/bevy/pull/10561
[10562]: https://github.com/bevyengine/bevy/pull/10562
[10563]: https://github.com/bevyengine/bevy/pull/10563
[10564]: https://github.com/bevyengine/bevy/pull/10564
[10565]: https://github.com/bevyengine/bevy/pull/10565
[10574]: https://github.com/bevyengine/bevy/pull/10574
[10577]: https://github.com/bevyengine/bevy/pull/10577
[10578]: https://github.com/bevyengine/bevy/pull/10578
[10580]: https://github.com/bevyengine/bevy/pull/10580
[10585]: https://github.com/bevyengine/bevy/pull/10585
[10588]: https://github.com/bevyengine/bevy/pull/10588
[10589]: https://github.com/bevyengine/bevy/pull/10589
[10591]: https://github.com/bevyengine/bevy/pull/10591
[10592]: https://github.com/bevyengine/bevy/pull/10592
[10594]: https://github.com/bevyengine/bevy/pull/10594
[10596]: https://github.com/bevyengine/bevy/pull/10596
[10598]: https://github.com/bevyengine/bevy/pull/10598
[10601]: https://github.com/bevyengine/bevy/pull/10601
[10608]: https://github.com/bevyengine/bevy/pull/10608
[10609]: https://github.com/bevyengine/bevy/pull/10609
[10610]: https://github.com/bevyengine/bevy/pull/10610
[10611]: https://github.com/bevyengine/bevy/pull/10611
[10614]: https://github.com/bevyengine/bevy/pull/10614
[10619]: https://github.com/bevyengine/bevy/pull/10619
[10620]: https://github.com/bevyengine/bevy/pull/10620
[10623]: https://github.com/bevyengine/bevy/pull/10623
[10624]: https://github.com/bevyengine/bevy/pull/10624
[10627]: https://github.com/bevyengine/bevy/pull/10627
[10628]: https://github.com/bevyengine/bevy/pull/10628
[10631]: https://github.com/bevyengine/bevy/pull/10631
[10632]: https://github.com/bevyengine/bevy/pull/10632
[10633]: https://github.com/bevyengine/bevy/pull/10633
[10638]: https://github.com/bevyengine/bevy/pull/10638
[10640]: https://github.com/bevyengine/bevy/pull/10640
[10643]: https://github.com/bevyengine/bevy/pull/10643
[10644]: https://github.com/bevyengine/bevy/pull/10644
[10646]: https://github.com/bevyengine/bevy/pull/10646
[10648]: https://github.com/bevyengine/bevy/pull/10648
[10650]: https://github.com/bevyengine/bevy/pull/10650
[10651]: https://github.com/bevyengine/bevy/pull/10651
[10653]: https://github.com/bevyengine/bevy/pull/10653
[10657]: https://github.com/bevyengine/bevy/pull/10657
[10660]: https://github.com/bevyengine/bevy/pull/10660
[10663]: https://github.com/bevyengine/bevy/pull/10663
[10670]: https://github.com/bevyengine/bevy/pull/10670
[10672]: https://github.com/bevyengine/bevy/pull/10672
[10674]: https://github.com/bevyengine/bevy/pull/10674
[10675]: https://github.com/bevyengine/bevy/pull/10675
[10681]: https://github.com/bevyengine/bevy/pull/10681
[10682]: https://github.com/bevyengine/bevy/pull/10682
[10683]: https://github.com/bevyengine/bevy/pull/10683
[10684]: https://github.com/bevyengine/bevy/pull/10684
[10691]: https://github.com/bevyengine/bevy/pull/10691
[10692]: https://github.com/bevyengine/bevy/pull/10692
[10693]: https://github.com/bevyengine/bevy/pull/10693
[10698]: https://github.com/bevyengine/bevy/pull/10698
[10699]: https://github.com/bevyengine/bevy/pull/10699
[10702]: https://github.com/bevyengine/bevy/pull/10702
[10705]: https://github.com/bevyengine/bevy/pull/10705
[10706]: https://github.com/bevyengine/bevy/pull/10706
[10712]: https://github.com/bevyengine/bevy/pull/10712
[10714]: https://github.com/bevyengine/bevy/pull/10714
[10719]: https://github.com/bevyengine/bevy/pull/10719
[10721]: https://github.com/bevyengine/bevy/pull/10721
[10722]: https://github.com/bevyengine/bevy/pull/10722
[10723]: https://github.com/bevyengine/bevy/pull/10723
[10730]: https://github.com/bevyengine/bevy/pull/10730
[10733]: https://github.com/bevyengine/bevy/pull/10733
[10734]: https://github.com/bevyengine/bevy/pull/10734
[10735]: https://github.com/bevyengine/bevy/pull/10735
[10736]: https://github.com/bevyengine/bevy/pull/10736
[10739]: https://github.com/bevyengine/bevy/pull/10739
[10740]: https://github.com/bevyengine/bevy/pull/10740
[10741]: https://github.com/bevyengine/bevy/pull/10741
[10742]: https://github.com/bevyengine/bevy/pull/10742
[10745]: https://github.com/bevyengine/bevy/pull/10745
[10746]: https://github.com/bevyengine/bevy/pull/10746
[10748]: https://github.com/bevyengine/bevy/pull/10748
[10749]: https://github.com/bevyengine/bevy/pull/10749
[10755]: https://github.com/bevyengine/bevy/pull/10755
[10758]: https://github.com/bevyengine/bevy/pull/10758
[10764]: https://github.com/bevyengine/bevy/pull/10764
[10770]: https://github.com/bevyengine/bevy/pull/10770
[10771]: https://github.com/bevyengine/bevy/pull/10771
[10774]: https://github.com/bevyengine/bevy/pull/10774
[10779]: https://github.com/bevyengine/bevy/pull/10779
[10780]: https://github.com/bevyengine/bevy/pull/10780
[10782]: https://github.com/bevyengine/bevy/pull/10782
[10785]: https://github.com/bevyengine/bevy/pull/10785
[10787]: https://github.com/bevyengine/bevy/pull/10787
[10788]: https://github.com/bevyengine/bevy/pull/10788
[10791]: https://github.com/bevyengine/bevy/pull/10791
[10799]: https://github.com/bevyengine/bevy/pull/10799
[10801]: https://github.com/bevyengine/bevy/pull/10801
[10804]: https://github.com/bevyengine/bevy/pull/10804
[10808]: https://github.com/bevyengine/bevy/pull/10808
[10811]: https://github.com/bevyengine/bevy/pull/10811
[10812]: https://github.com/bevyengine/bevy/pull/10812
[10815]: https://github.com/bevyengine/bevy/pull/10815
[10818]: https://github.com/bevyengine/bevy/pull/10818
[10822]: https://github.com/bevyengine/bevy/pull/10822
[10827]: https://github.com/bevyengine/bevy/pull/10827
[10836]: https://github.com/bevyengine/bevy/pull/10836
[10837]: https://github.com/bevyengine/bevy/pull/10837
[10841]: https://github.com/bevyengine/bevy/pull/10841
[10847]: https://github.com/bevyengine/bevy/pull/10847
[10848]: https://github.com/bevyengine/bevy/pull/10848
[10854]: https://github.com/bevyengine/bevy/pull/10854
[10856]: https://github.com/bevyengine/bevy/pull/10856
[10857]: https://github.com/bevyengine/bevy/pull/10857
[10859]: https://github.com/bevyengine/bevy/pull/10859
[10872]: https://github.com/bevyengine/bevy/pull/10872
[10873]: https://github.com/bevyengine/bevy/pull/10873
[10878]: https://github.com/bevyengine/bevy/pull/10878
[10879]: https://github.com/bevyengine/bevy/pull/10879
[10882]: https://github.com/bevyengine/bevy/pull/10882
[10884]: https://github.com/bevyengine/bevy/pull/10884
[10885]: https://github.com/bevyengine/bevy/pull/10885
[10889]: https://github.com/bevyengine/bevy/pull/10889
[10892]: https://github.com/bevyengine/bevy/pull/10892
[10893]: https://github.com/bevyengine/bevy/pull/10893
[10897]: https://github.com/bevyengine/bevy/pull/10897
[10905]: https://github.com/bevyengine/bevy/pull/10905
[10906]: https://github.com/bevyengine/bevy/pull/10906
[10910]: https://github.com/bevyengine/bevy/pull/10910
[10911]: https://github.com/bevyengine/bevy/pull/10911
[10914]: https://github.com/bevyengine/bevy/pull/10914
[10918]: https://github.com/bevyengine/bevy/pull/10918
[10920]: https://github.com/bevyengine/bevy/pull/10920
[10922]: https://github.com/bevyengine/bevy/pull/10922
[10927]: https://github.com/bevyengine/bevy/pull/10927
[10930]: https://github.com/bevyengine/bevy/pull/10930
[10937]: https://github.com/bevyengine/bevy/pull/10937
[10939]: https://github.com/bevyengine/bevy/pull/10939
[10941]: https://github.com/bevyengine/bevy/pull/10941
[10942]: https://github.com/bevyengine/bevy/pull/10942
[10943]: https://github.com/bevyengine/bevy/pull/10943
[10946]: https://github.com/bevyengine/bevy/pull/10946
[10947]: https://github.com/bevyengine/bevy/pull/10947
[10951]: https://github.com/bevyengine/bevy/pull/10951
[10961]: https://github.com/bevyengine/bevy/pull/10961
[10962]: https://github.com/bevyengine/bevy/pull/10962
[10963]: https://github.com/bevyengine/bevy/pull/10963
[10965]: https://github.com/bevyengine/bevy/pull/10965
[10966]: https://github.com/bevyengine/bevy/pull/10966
[10967]: https://github.com/bevyengine/bevy/pull/10967
[10969]: https://github.com/bevyengine/bevy/pull/10969
[10970]: https://github.com/bevyengine/bevy/pull/10970
[10971]: https://github.com/bevyengine/bevy/pull/10971
[10977]: https://github.com/bevyengine/bevy/pull/10977
[10979]: https://github.com/bevyengine/bevy/pull/10979
[10980]: https://github.com/bevyengine/bevy/pull/10980
[10988]: https://github.com/bevyengine/bevy/pull/10988
[10989]: https://github.com/bevyengine/bevy/pull/10989
[10990]: https://github.com/bevyengine/bevy/pull/10990
[10996]: https://github.com/bevyengine/bevy/pull/10996
[10998]: https://github.com/bevyengine/bevy/pull/10998
[10999]: https://github.com/bevyengine/bevy/pull/10999
[11002]: https://github.com/bevyengine/bevy/pull/11002
[11003]: https://github.com/bevyengine/bevy/pull/11003
[11006]: https://github.com/bevyengine/bevy/pull/11006
[11010]: https://github.com/bevyengine/bevy/pull/11010
[11011]: https://github.com/bevyengine/bevy/pull/11011
[11012]: https://github.com/bevyengine/bevy/pull/11012
[11014]: https://github.com/bevyengine/bevy/pull/11014
[11016]: https://github.com/bevyengine/bevy/pull/11016
[11017]: https://github.com/bevyengine/bevy/pull/11017
[11018]: https://github.com/bevyengine/bevy/pull/11018
[11020]: https://github.com/bevyengine/bevy/pull/11020
[11021]: https://github.com/bevyengine/bevy/pull/11021
[11023]: https://github.com/bevyengine/bevy/pull/11023
[11024]: https://github.com/bevyengine/bevy/pull/11024
[11025]: https://github.com/bevyengine/bevy/pull/11025
[11027]: https://github.com/bevyengine/bevy/pull/11027
[11028]: https://github.com/bevyengine/bevy/pull/11028
[11029]: https://github.com/bevyengine/bevy/pull/11029
[11030]: https://github.com/bevyengine/bevy/pull/11030
[11031]: https://github.com/bevyengine/bevy/pull/11031
[11037]: https://github.com/bevyengine/bevy/pull/11037
[11040]: https://github.com/bevyengine/bevy/pull/11040
[11041]: https://github.com/bevyengine/bevy/pull/11041
[11043]: https://github.com/bevyengine/bevy/pull/11043
[11045]: https://github.com/bevyengine/bevy/pull/11045
[11049]: https://github.com/bevyengine/bevy/pull/11049
[11051]: https://github.com/bevyengine/bevy/pull/11051
[11053]: https://github.com/bevyengine/bevy/pull/11053
[11054]: https://github.com/bevyengine/bevy/pull/11054
[11057]: https://github.com/bevyengine/bevy/pull/11057
[11058]: https://github.com/bevyengine/bevy/pull/11058
[11059]: https://github.com/bevyengine/bevy/pull/11059
[11060]: https://github.com/bevyengine/bevy/pull/11060
[11063]: https://github.com/bevyengine/bevy/pull/11063
[11064]: https://github.com/bevyengine/bevy/pull/11064
[11065]: https://github.com/bevyengine/bevy/pull/11065
[11068]: https://github.com/bevyengine/bevy/pull/11068
[11069]: https://github.com/bevyengine/bevy/pull/11069
[11071]: https://github.com/bevyengine/bevy/pull/11071
[11072]: https://github.com/bevyengine/bevy/pull/11072
[11075]: https://github.com/bevyengine/bevy/pull/11075
[11076]: https://github.com/bevyengine/bevy/pull/11076
[11077]: https://github.com/bevyengine/bevy/pull/11077
[11080]: https://github.com/bevyengine/bevy/pull/11080
[11082]: https://github.com/bevyengine/bevy/pull/11082
[11084]: https://github.com/bevyengine/bevy/pull/11084
[11089]: https://github.com/bevyengine/bevy/pull/11089
[11092]: https://github.com/bevyengine/bevy/pull/11092
[11095]: https://github.com/bevyengine/bevy/pull/11095
[11103]: https://github.com/bevyengine/bevy/pull/11103
[11104]: https://github.com/bevyengine/bevy/pull/11104
[11106]: https://github.com/bevyengine/bevy/pull/11106
[11107]: https://github.com/bevyengine/bevy/pull/11107
[11110]: https://github.com/bevyengine/bevy/pull/11110
[11118]: https://github.com/bevyengine/bevy/pull/11118
[11128]: https://github.com/bevyengine/bevy/pull/11128
[11129]: https://github.com/bevyengine/bevy/pull/11129
[11134]: https://github.com/bevyengine/bevy/pull/11134
[11138]: https://github.com/bevyengine/bevy/pull/11138
[11139]: https://github.com/bevyengine/bevy/pull/11139
[11140]: https://github.com/bevyengine/bevy/pull/11140
[11142]: https://github.com/bevyengine/bevy/pull/11142
[11143]: https://github.com/bevyengine/bevy/pull/11143
[11146]: https://github.com/bevyengine/bevy/pull/11146
[11149]: https://github.com/bevyengine/bevy/pull/11149
[11152]: https://github.com/bevyengine/bevy/pull/11152
[11153]: https://github.com/bevyengine/bevy/pull/11153
[11162]: https://github.com/bevyengine/bevy/pull/11162
[11163]: https://github.com/bevyengine/bevy/pull/11163
[11164]: https://github.com/bevyengine/bevy/pull/11164
[11166]: https://github.com/bevyengine/bevy/pull/11166
[11167]: https://github.com/bevyengine/bevy/pull/11167
[11170]: https://github.com/bevyengine/bevy/pull/11170
[11171]: https://github.com/bevyengine/bevy/pull/11171
[11172]: https://github.com/bevyengine/bevy/pull/11172
[11176]: https://github.com/bevyengine/bevy/pull/11176
[11178]: https://github.com/bevyengine/bevy/pull/11178
[11179]: https://github.com/bevyengine/bevy/pull/11179
[11180]: https://github.com/bevyengine/bevy/pull/11180
[11182]: https://github.com/bevyengine/bevy/pull/11182
[11188]: https://github.com/bevyengine/bevy/pull/11188
[11189]: https://github.com/bevyengine/bevy/pull/11189
[11191]: https://github.com/bevyengine/bevy/pull/11191
[11192]: https://github.com/bevyengine/bevy/pull/11192
[11193]: https://github.com/bevyengine/bevy/pull/11193
[11194]: https://github.com/bevyengine/bevy/pull/11194
[11195]: https://github.com/bevyengine/bevy/pull/11195
[11199]: https://github.com/bevyengine/bevy/pull/11199
[11205]: https://github.com/bevyengine/bevy/pull/11205
[11206]: https://github.com/bevyengine/bevy/pull/11206
[11212]: https://github.com/bevyengine/bevy/pull/11212
[11218]: https://github.com/bevyengine/bevy/pull/11218
[11226]: https://github.com/bevyengine/bevy/pull/11226
[11227]: https://github.com/bevyengine/bevy/pull/11227
[11234]: https://github.com/bevyengine/bevy/pull/11234
[11238]: https://github.com/bevyengine/bevy/pull/11238
[11239]: https://github.com/bevyengine/bevy/pull/11239
[11242]: https://github.com/bevyengine/bevy/pull/11242
[11243]: https://github.com/bevyengine/bevy/pull/11243
[11245]: https://github.com/bevyengine/bevy/pull/11245
[11248]: https://github.com/bevyengine/bevy/pull/11248
[11250]: https://github.com/bevyengine/bevy/pull/11250
[11254]: https://github.com/bevyengine/bevy/pull/11254
[11260]: https://github.com/bevyengine/bevy/pull/11260
[11261]: https://github.com/bevyengine/bevy/pull/11261
[11268]: https://github.com/bevyengine/bevy/pull/11268
[11269]: https://github.com/bevyengine/bevy/pull/11269
[11270]: https://github.com/bevyengine/bevy/pull/11270
[11280]: https://github.com/bevyengine/bevy/pull/11280
[11284]: https://github.com/bevyengine/bevy/pull/11284
[11289]: https://github.com/bevyengine/bevy/pull/11289
[11292]: https://github.com/bevyengine/bevy/pull/11292
[11293]: https://github.com/bevyengine/bevy/pull/11293
[11296]: https://github.com/bevyengine/bevy/pull/11296
[11305]: https://github.com/bevyengine/bevy/pull/11305
[11306]: https://github.com/bevyengine/bevy/pull/11306
[11307]: https://github.com/bevyengine/bevy/pull/11307
[11310]: https://github.com/bevyengine/bevy/pull/11310
[11313]: https://github.com/bevyengine/bevy/pull/11313
[11316]: https://github.com/bevyengine/bevy/pull/11316
[11319]: https://github.com/bevyengine/bevy/pull/11319
[11321]: https://github.com/bevyengine/bevy/pull/11321
[11323]: https://github.com/bevyengine/bevy/pull/11323
[11325]: https://github.com/bevyengine/bevy/pull/11325
[11326]: https://github.com/bevyengine/bevy/pull/11326
[11327]: https://github.com/bevyengine/bevy/pull/11327
[11330]: https://github.com/bevyengine/bevy/pull/11330
[11331]: https://github.com/bevyengine/bevy/pull/11331
[11332]: https://github.com/bevyengine/bevy/pull/11332
[11334]: https://github.com/bevyengine/bevy/pull/11334
[11335]: https://github.com/bevyengine/bevy/pull/11335
[11336]: https://github.com/bevyengine/bevy/pull/11336
[11338]: https://github.com/bevyengine/bevy/pull/11338
[11342]: https://github.com/bevyengine/bevy/pull/11342
[11347]: https://github.com/bevyengine/bevy/pull/11347
[11353]: https://github.com/bevyengine/bevy/pull/11353
[11360]: https://github.com/bevyengine/bevy/pull/11360
[11361]: https://github.com/bevyengine/bevy/pull/11361
[11366]: https://github.com/bevyengine/bevy/pull/11366
[11368]: https://github.com/bevyengine/bevy/pull/11368
[11369]: https://github.com/bevyengine/bevy/pull/11369
[11370]: https://github.com/bevyengine/bevy/pull/11370
[11371]: https://github.com/bevyengine/bevy/pull/11371
[11373]: https://github.com/bevyengine/bevy/pull/11373
[11379]: https://github.com/bevyengine/bevy/pull/11379
[11381]: https://github.com/bevyengine/bevy/pull/11381
[11383]: https://github.com/bevyengine/bevy/pull/11383
[11386]: https://github.com/bevyengine/bevy/pull/11386
[11388]: https://github.com/bevyengine/bevy/pull/11388
[11389]: https://github.com/bevyengine/bevy/pull/11389
[11391]: https://github.com/bevyengine/bevy/pull/11391
[11397]: https://github.com/bevyengine/bevy/pull/11397
[11399]: https://github.com/bevyengine/bevy/pull/11399
[11400]: https://github.com/bevyengine/bevy/pull/11400
[11403]: https://github.com/bevyengine/bevy/pull/11403
[11404]: https://github.com/bevyengine/bevy/pull/11404
[11405]: https://github.com/bevyengine/bevy/pull/11405
[11407]: https://github.com/bevyengine/bevy/pull/11407
[11412]: https://github.com/bevyengine/bevy/pull/11412
[11416]: https://github.com/bevyengine/bevy/pull/11416
[11417]: https://github.com/bevyengine/bevy/pull/11417
[11418]: https://github.com/bevyengine/bevy/pull/11418
[11419]: https://github.com/bevyengine/bevy/pull/11419
[11420]: https://github.com/bevyengine/bevy/pull/11420
[11421]: https://github.com/bevyengine/bevy/pull/11421
[11422]: https://github.com/bevyengine/bevy/pull/11422
[11425]: https://github.com/bevyengine/bevy/pull/11425
[11428]: https://github.com/bevyengine/bevy/pull/11428
[11431]: https://github.com/bevyengine/bevy/pull/11431
[11432]: https://github.com/bevyengine/bevy/pull/11432
[11433]: https://github.com/bevyengine/bevy/pull/11433
[11434]: https://github.com/bevyengine/bevy/pull/11434
[11435]: https://github.com/bevyengine/bevy/pull/11435
[11436]: https://github.com/bevyengine/bevy/pull/11436
[11437]: https://github.com/bevyengine/bevy/pull/11437
[11439]: https://github.com/bevyengine/bevy/pull/11439
[11440]: https://github.com/bevyengine/bevy/pull/11440
[11442]: https://github.com/bevyengine/bevy/pull/11442
[11444]: https://github.com/bevyengine/bevy/pull/11444
[11445]: https://github.com/bevyengine/bevy/pull/11445
[11447]: https://github.com/bevyengine/bevy/pull/11447
[11454]: https://github.com/bevyengine/bevy/pull/11454
[11455]: https://github.com/bevyengine/bevy/pull/11455
[11456]: https://github.com/bevyengine/bevy/pull/11456
[11461]: https://github.com/bevyengine/bevy/pull/11461
[11462]: https://github.com/bevyengine/bevy/pull/11462
[11467]: https://github.com/bevyengine/bevy/pull/11467
[11469]: https://github.com/bevyengine/bevy/pull/11469
[11474]: https://github.com/bevyengine/bevy/pull/11474
[11480]: https://github.com/bevyengine/bevy/pull/11480
[11483]: https://github.com/bevyengine/bevy/pull/11483
[11484]: https://github.com/bevyengine/bevy/pull/11484
[11486]: https://github.com/bevyengine/bevy/pull/11486
[11487]: https://github.com/bevyengine/bevy/pull/11487
[11489]: https://github.com/bevyengine/bevy/pull/11489
[11491]: https://github.com/bevyengine/bevy/pull/11491
[11497]: https://github.com/bevyengine/bevy/pull/11497
[11498]: https://github.com/bevyengine/bevy/pull/11498
[11499]: https://github.com/bevyengine/bevy/pull/11499
[11500]: https://github.com/bevyengine/bevy/pull/11500
[11504]: https://github.com/bevyengine/bevy/pull/11504
[11506]: https://github.com/bevyengine/bevy/pull/11506
[11507]: https://github.com/bevyengine/bevy/pull/11507
[11508]: https://github.com/bevyengine/bevy/pull/11508
[11512]: https://github.com/bevyengine/bevy/pull/11512
[11513]: https://github.com/bevyengine/bevy/pull/11513
[11514]: https://github.com/bevyengine/bevy/pull/11514
[11519]: https://github.com/bevyengine/bevy/pull/11519
[11521]: https://github.com/bevyengine/bevy/pull/11521
[11523]: https://github.com/bevyengine/bevy/pull/11523
[11524]: https://github.com/bevyengine/bevy/pull/11524
[11526]: https://github.com/bevyengine/bevy/pull/11526
[11527]: https://github.com/bevyengine/bevy/pull/11527
[11528]: https://github.com/bevyengine/bevy/pull/11528
[11529]: https://github.com/bevyengine/bevy/pull/11529
[11531]: https://github.com/bevyengine/bevy/pull/11531
[11534]: https://github.com/bevyengine/bevy/pull/11534
[11538]: https://github.com/bevyengine/bevy/pull/11538
[11540]: https://github.com/bevyengine/bevy/pull/11540
[11541]: https://github.com/bevyengine/bevy/pull/11541
[11543]: https://github.com/bevyengine/bevy/pull/11543
[11548]: https://github.com/bevyengine/bevy/pull/11548
[11551]: https://github.com/bevyengine/bevy/pull/11551
[11555]: https://github.com/bevyengine/bevy/pull/11555
[11556]: https://github.com/bevyengine/bevy/pull/11556
[11560]: https://github.com/bevyengine/bevy/pull/11560
[11561]: https://github.com/bevyengine/bevy/pull/11561
[11573]: https://github.com/bevyengine/bevy/pull/11573
[11574]: https://github.com/bevyengine/bevy/pull/11574
[11575]: https://github.com/bevyengine/bevy/pull/11575
[11576]: https://github.com/bevyengine/bevy/pull/11576
[11578]: https://github.com/bevyengine/bevy/pull/11578
[11580]: https://github.com/bevyengine/bevy/pull/11580
[11581]: https://github.com/bevyengine/bevy/pull/11581
[11583]: https://github.com/bevyengine/bevy/pull/11583
[11585]: https://github.com/bevyengine/bevy/pull/11585
[11586]: https://github.com/bevyengine/bevy/pull/11586
[11591]: https://github.com/bevyengine/bevy/pull/11591
[11596]: https://github.com/bevyengine/bevy/pull/11596
[11597]: https://github.com/bevyengine/bevy/pull/11597
[11599]: https://github.com/bevyengine/bevy/pull/11599
[11600]: https://github.com/bevyengine/bevy/pull/11600
[11604]: https://github.com/bevyengine/bevy/pull/11604
[11605]: https://github.com/bevyengine/bevy/pull/11605
[11610]: https://github.com/bevyengine/bevy/pull/11610
[11611]: https://github.com/bevyengine/bevy/pull/11611
[11612]: https://github.com/bevyengine/bevy/pull/11612
[11615]: https://github.com/bevyengine/bevy/pull/11615
[11616]: https://github.com/bevyengine/bevy/pull/11616
[11617]: https://github.com/bevyengine/bevy/pull/11617
[11618]: https://github.com/bevyengine/bevy/pull/11618
[11622]: https://github.com/bevyengine/bevy/pull/11622
[11626]: https://github.com/bevyengine/bevy/pull/11626
[11627]: https://github.com/bevyengine/bevy/pull/11627
[11630]: https://github.com/bevyengine/bevy/pull/11630
[11635]: https://github.com/bevyengine/bevy/pull/11635
[11639]: https://github.com/bevyengine/bevy/pull/11639
[11640]: https://github.com/bevyengine/bevy/pull/11640
[11641]: https://github.com/bevyengine/bevy/pull/11641
[11644]: https://github.com/bevyengine/bevy/pull/11644
[11645]: https://github.com/bevyengine/bevy/pull/11645
[11649]: https://github.com/bevyengine/bevy/pull/11649
[11650]: https://github.com/bevyengine/bevy/pull/11650
[11652]: https://github.com/bevyengine/bevy/pull/11652
[11660]: https://github.com/bevyengine/bevy/pull/11660
[11662]: https://github.com/bevyengine/bevy/pull/11662
[11664]: https://github.com/bevyengine/bevy/pull/11664
[11666]: https://github.com/bevyengine/bevy/pull/11666
[11669]: https://github.com/bevyengine/bevy/pull/11669
[11671]: https://github.com/bevyengine/bevy/pull/11671
[11672]: https://github.com/bevyengine/bevy/pull/11672
[11675]: https://github.com/bevyengine/bevy/pull/11675
[11676]: https://github.com/bevyengine/bevy/pull/11676
[11678]: https://github.com/bevyengine/bevy/pull/11678
[11684]: https://github.com/bevyengine/bevy/pull/11684
[11686]: https://github.com/bevyengine/bevy/pull/11686
[11687]: https://github.com/bevyengine/bevy/pull/11687
[11688]: https://github.com/bevyengine/bevy/pull/11688
[11690]: https://github.com/bevyengine/bevy/pull/11690
[11693]: https://github.com/bevyengine/bevy/pull/11693
[11697]: https://github.com/bevyengine/bevy/pull/11697
[11699]: https://github.com/bevyengine/bevy/pull/11699
[11700]: https://github.com/bevyengine/bevy/pull/11700
[11703]: https://github.com/bevyengine/bevy/pull/11703
[11705]: https://github.com/bevyengine/bevy/pull/11705
[11709]: https://github.com/bevyengine/bevy/pull/11709
[11710]: https://github.com/bevyengine/bevy/pull/11710
[11712]: https://github.com/bevyengine/bevy/pull/11712
[11720]: https://github.com/bevyengine/bevy/pull/11720
[11721]: https://github.com/bevyengine/bevy/pull/11721
[11722]: https://github.com/bevyengine/bevy/pull/11722
[11725]: https://github.com/bevyengine/bevy/pull/11725
[11726]: https://github.com/bevyengine/bevy/pull/11726
[11728]: https://github.com/bevyengine/bevy/pull/11728
[11733]: https://github.com/bevyengine/bevy/pull/11733
[11735]: https://github.com/bevyengine/bevy/pull/11735
[11736]: https://github.com/bevyengine/bevy/pull/11736
[11737]: https://github.com/bevyengine/bevy/pull/11737
[11745]: https://github.com/bevyengine/bevy/pull/11745
[11747]: https://github.com/bevyengine/bevy/pull/11747
[11751]: https://github.com/bevyengine/bevy/pull/11751
[11758]: https://github.com/bevyengine/bevy/pull/11758
[11764]: https://github.com/bevyengine/bevy/pull/11764
[11767]: https://github.com/bevyengine/bevy/pull/11767
[11769]: https://github.com/bevyengine/bevy/pull/11769
[11773]: https://github.com/bevyengine/bevy/pull/11773
[11777]: https://github.com/bevyengine/bevy/pull/11777
[11780]: https://github.com/bevyengine/bevy/pull/11780
[11781]: https://github.com/bevyengine/bevy/pull/11781
[11783]: https://github.com/bevyengine/bevy/pull/11783
[11785]: https://github.com/bevyengine/bevy/pull/11785
[11791]: https://github.com/bevyengine/bevy/pull/11791
[11792]: https://github.com/bevyengine/bevy/pull/11792
[11795]: https://github.com/bevyengine/bevy/pull/11795
[11797]: https://github.com/bevyengine/bevy/pull/11797
[11798]: https://github.com/bevyengine/bevy/pull/11798
[11800]: https://github.com/bevyengine/bevy/pull/11800
[11801]: https://github.com/bevyengine/bevy/pull/11801
[11803]: https://github.com/bevyengine/bevy/pull/11803
[11805]: https://github.com/bevyengine/bevy/pull/11805
[11810]: https://github.com/bevyengine/bevy/pull/11810
[11818]: https://github.com/bevyengine/bevy/pull/11818
[11822]: https://github.com/bevyengine/bevy/pull/11822
[11832]: https://github.com/bevyengine/bevy/pull/11832
[11838]: https://github.com/bevyengine/bevy/pull/11838
[11847]: https://github.com/bevyengine/bevy/pull/11847
[11850]: https://github.com/bevyengine/bevy/pull/11850
[11855]: https://github.com/bevyengine/bevy/pull/11855
[11856]: https://github.com/bevyengine/bevy/pull/11856
[11860]: https://github.com/bevyengine/bevy/pull/11860
[11865]: https://github.com/bevyengine/bevy/pull/11865
[11866]: https://github.com/bevyengine/bevy/pull/11866
[11867]: https://github.com/bevyengine/bevy/pull/11867
[11868]: https://github.com/bevyengine/bevy/pull/11868
[11870]: https://github.com/bevyengine/bevy/pull/11870
[11878]: https://github.com/bevyengine/bevy/pull/11878
[11880]: https://github.com/bevyengine/bevy/pull/11880
[11882]: https://github.com/bevyengine/bevy/pull/11882
[11884]: https://github.com/bevyengine/bevy/pull/11884
[11889]: https://github.com/bevyengine/bevy/pull/11889
[11893]: https://github.com/bevyengine/bevy/pull/11893
[11894]: https://github.com/bevyengine/bevy/pull/11894
[11907]: https://github.com/bevyengine/bevy/pull/11907
[11909]: https://github.com/bevyengine/bevy/pull/11909
[11910]: https://github.com/bevyengine/bevy/pull/11910
[11911]: https://github.com/bevyengine/bevy/pull/11911
[11913]: https://github.com/bevyengine/bevy/pull/11913
[11914]: https://github.com/bevyengine/bevy/pull/11914
[11915]: https://github.com/bevyengine/bevy/pull/11915

## Version 0.12.0 (2023-11-04)

### A-ECS + A-Diagnostics

- [Cache parallel iteration spans][9950]

### A-ECS + A-Scenes

- [Make builder types take and return `Self`][10001]

### A-Scenes

- [Move scene spawner systems to SpawnScene schedule][9260]
- [Add `SceneInstanceReady`][9313]
- [Add `SpawnScene` to prelude][9451]
- [Finish documenting `bevy_scene`][9949]
- [Only attempt to copy resources that still exist from scenes][9984]
- [Correct Scene loader error description][10161]

### A-Tasks + A-Diagnostics

- [Fix doc warning in bevy_tasks][9348]

### A-Tasks

- [elaborate on TaskPool and bevy tasks][8750]
- [Remove Resource and add Debug to TaskPoolOptions][9485]
- [Fix clippy lint in single_threaded_task_pool][9851]
- [Remove dependecies from bevy_tasks' README][9881]
- [Allow using async_io::block_on in bevy_tasks][9626]
- [add test for nested scopes][10026]
- [Global TaskPool API improvements][10008]

### A-Audio + A-Windowing

- [Application lifetime events (suspend audio on Android)][10158]

### A-Animation + A-Transform

- [Add system parameter for computing up-to-date `GlobalTransform`s][8603]

### A-Transform

- [Update `GlobalTransform` on insertion][9081]
- [Add `Without<Parent>` filter to `sync_simple_transforms`' orphaned entities query][9518]
- [Fix ambiguities in transform example][9845]

### A-App

- [Add `track_caller` to `App::add_plugins`][9174]
- [Remove redundant check for `AppExit` events in `ScheduleRunnerPlugin`][9421]
- [fix typos in crates/bevy_app/src/app.rs][10173]
- [fix typos in crates/bevy_app/src/app.rs][10173]
- [fix run-once runners][10195]

### A-ECS + A-App

- [Add configure_schedules to App and Schedules to apply `ScheduleBuildSettings` to all schedules][9514]
- [Only run event systems if they have tangible work to do][7728]

### A-Rendering + A-Gizmos

- [Fix gizmo draw order in 2D][9129]
- [Fix gizmo line width issue when using perspective][9067]

### A-Rendering + A-Diagnostics

- [Include note of common profiling issue][9484]
- [Enhance many_cubes stress test use cases][9596]
- [GLTF loader: handle warning NODE_SKINNED_MESH_WITHOUT_SKIN][9360]

### A-Rendering + A-Reflection

- [Register `AlphaMode` type][9222]

### A-Windowing

- [Add option to toggle window control buttons][9083]
- [Fixed: Default window is now "App" instead of "Bevy App"][9301]
- [improve documentation relating to `WindowPlugin` and `Window`][9173]
- [Improve `bevy_winit` documentation][7609]
- [Change `WinitPlugin` defaults to limit game update rate when window is not visible][7611]
- [User controlled window visibility][9355]
- [Check cursor position for out of bounds of the window][8855]
- [Fix doc link in transparent_window example][9697]
- [Wait before making window visible][9692]
- [don't create windows on winit StartCause::Init event][9684]
- [Fix the doc warning attribute and document remaining items for `bevy_window`][9933]
- [Revert "macOS Sonoma (14.0) / Xcode 15.0 — Compatibility Fixes + Docs…][9991]
- [Revert "macOS Sonoma (14.0) / Xcode 15.0 — Compatibility Fixes + Docs…][9991]
- [Allow Bevy to start from non-main threads on supported platforms][10020]
- [Prevent black frames during startup][9826]
- [Slightly improve `CursorIcon` doc.][10289]
- [Fix typo in window.rs][10358]

### A-Gizmos

- [Replace AHash with a good sequence for entity AABB colors][9175]
- [gizmo plugin lag bugfix][9166]
- [Clarify immediate mode in `Gizmos` documentation][9183]
- [Fix crash when drawing line gizmo with less than 2 vertices][9101]
- [Document that gizmo `depth_bias` has no effect in 2D][10074]

### A-Utils

- [change 'collapse_type_name' to retain enum types][9587]
- [bevy_derive: Fix `#[deref]` breaking other attributes][9551]
- [Move default docs][9638]

### A-Rendering + A-Assets

- [Import the second UV map if present in glTF files.][9992]
- [fix custom shader imports][10030]
- [Add `ImageSamplerDescriptor` as an image loader setting][9982]

### A-ECS

- [Add the Has world query to bevy_ecs::prelude][9204]
- [Simplify parallel iteration methods][8854]
- [Fix safety invariants for `WorldQuery::fetch` and simplify cloning][8246]
- [Derive debug for ManualEventIterator][9293]
- [Add `EntityMap::clear`][9291]
- [Add a paragraph to the lifetimeless module doc][9312]
- [opt-out `multi-threaded` feature flag][9269]
- [Fix `ambiguous_with` breaking run conditions][9253]
- [Add `RunSystem`][9366]
- [Add `replace_if_neq` to `DetectChangesMut`][9418]
- [Adding `Copy, Clone, Debug` to derived traits of `ExecutorKind`][9385]
- [Fix incorrect documentation link in `DetectChangesMut`][9431]
- [Implement `Debug` for `UnsafeWorldCell`][9460]
- [Relax In/Out bounds on impl Debug for dyn System][9581]
- [Improve various `Debug` implementations][9588]
- [Make `run_if_inner` public and rename to `run_if_dyn`][9576]
- [Refactor build_schedule and related errors][9579]
- [Add `system.map(...)` for transforming the output of a system][8526]
- [Reorganize `Events` and `EventSequence` code][9306]
- [Replaced EntityMap with HashMap][9461]
- [clean up configure_set(s) erroring][9577]
- [Relax more `Sync` bounds on `Local`][9589]
- [Rename `ManualEventIterator`][9592]
- [Replaced `EntityCommand` Implementation for `FnOnce`][9604]
- [Add a variant of `Events::update` that returns the removed events][9542]
- [Move schedule name into `Schedule`][9600]
- [port old ambiguity tests over][9617]
- [Refactor `EventReader::iter` to `read`][9631]
- [fix ambiguity reporting][9648]
- [Fix anonymous set name stack overflow][9650]
- [Fix unsoundness in `QueryState::is_empty`][9463]
- [Add panicking helpers for getting components from `Query`][9659]
- [Replace `IntoSystemSetConfig` with `IntoSystemSetConfigs`][9247]
- [Moved `get_component(_unchecked_mut)` from `Query` to `QueryState`][9686]
- [Fix naming on "tick" Column and ComponentSparseSet methods][9744]
- [Clarify a comment in Option WorldQuery impl][9749]
- [Return a boolean from `set_if_neq`][9801]
- [Rename RemovedComponents::iter/iter_with_id to read/read_with_id][9778]
- [Remove some old references to CoreSet][9833]
- [Use single threaded executor for archetype benches][9835]
- [docs: Improve some `ComponentId` doc cross-linking.][9839]
- [One Shot Systems][8963]
- [Add mutual exclusion safety info on filter_fetch][9836]
- [add try_insert to entity commands][9844]
- [Improve codegen for world validation][9464]
- [docs: Use intradoc links for method references.][9958]
- [Remove States::variants and remove enum-only restriction its derive][9945]
- [`as_deref_mut()` method for Mut-like types][9912]
- [refactor: Change `Option<With<T>>` query params to `Has<T>`][9959]
- [Hide `UnsafeWorldCell::unsafe_world`][9741]
- [Add a public API to ArchetypeGeneration/Id][9825]
- [Ignore ambiguous components or resources][9895]
- [Use chain in breakout example][10124]
- [`ParamSet`s containing non-send parameters should also be non-send][10211]
- [Replace all labels with interned labels][7762]
- [Fix outdated comment referencing CoreSet][10294]

### A-Rendering + A-Math

- [derive Clone/Copy/Debug trio for shape::Cylinder][9705]

### A-UI

- [Fix for vertical text bounds and alignment][9133]
- [UI extraction order fix][9099]
- [Update text example using default font][9259]
- [bevy_ui: fix doc formatting for some Style fields][9295]
- [Remove the `With<Parent>` query filter from `bevy_ui::render::extract_uinode_borders`][9285]
- [Fix incorrent doc comment for the set method of `ContentSize`][9345]
- [Improved text widget doc comments][9344]
- [Change the default for the `measure_func` field of `ContentSize` to None.][9346]
- [Unnecessary line in game_menu example][9406]
- [Change `UiScale` to a tuple struct][9444]
- [Remove unnecessary doc string][9481]
- [Add some missing pub in ui_node][9529]
- [UI examples clean up][9479]
- [`round_ties_up` fix][9548]
- [fix incorrect docs for `JustifyItems` and `JustifySelf`][9539]
- [Added `Val::ZERO` Constant][9566]
- [Cleanup some bevy_text pipeline.rs][9111]
- [Make `GridPlacement`'s fields non-zero and add accessor functions.][9486]
- [Remove `Val`'s `try_*` arithmetic methods][9609]
- [UI node bundle comment fix][9404]
- [Do not panic on non-UI child of UI entity][9621]
- [Rename `Val` `evaluate` to `resolve` and implement viewport variant support][9568]
- [Change `Urect::width` & `Urect::height` to be const][9640]
- [`TextLayoutInfo::size` should hold the drawn size of the text, and not a scaled value.][7794]
- [`impl From<String>` and `From<&str>` for `TextSection`][8856]
- [Remove z-axis scaling in `extract_text2d_sprite`][9733]
- [Fix doc comments for align items][9739]
- [Add tests to `bevy_ui::Layout`][9781]
- [examples: Remove unused doc comments.][9795]
- [Add missing `bevy_text` feature attribute to `TextBundle` from impl][9785]
- [Move `Val` into `geometry`][9818]
- [Derive Serialize and Deserialize for UiRect][9820]
- [`ContentSize` replacement fix][9753]
- [Round UI coordinates after scaling][9784]
- [Have a separate implicit viewport node per root node + make viewport node `Display::Grid`][9637]
- [Rename `num_font_atlases`  to `len`.][9879]
- [Fix documentation for ui node Style][9935]
- [`text_wrap_debug` scale factor commandline args][9951]
- [Store both the rounded and unrounded node size in Node][9923]
- [Various accessibility API updates.][9989]
- [UI node outlines][9931]
- [Implement serialize and deserialize for some UI types][10044]
- [Tidy up UI node docs][10189]
- [Remove unused import warning when default_font feature is disabled][10230]
- [Fix crash with certain right-aligned text][10271]
- [Add some more docs for bevy_text.][9873]
- [Implement `Neg` for `Val`][10295]
- [`normalize` method for `Rect`][10297]
- [don't Implement `Display` for `Val`][10345]
- [[bevy_text] Document what happens when font is not specified][10252]
- [Update UI alignment docs][10303]
- [Add stack index to `Node`][9853]
- [don't Implement `Display` for `Val`][10345]

### A-Animation

- [Fix doc typo][9162]
- [Expose `animation_clip` paths][9392]
- [animations: convert skinning weights from unorm8x4 to float32x4][9338]
- [API updates to the AnimationPlayer][9002]
- [only take up to the max number of joints][9351]
- [check root node for animations][9407]
- [Fix morph interpolation][9927]

### A-Pointers

- [Put `#[repr(transparent)]` attr to bevy_ptr types][9068]

### A-Assets + A-Reflection

- [reflect: `TypePath` part 2][8768]

### A-Rendering + A-Hierarchy

- [default inherited visibility when parent has invalid components][10275]

### A-ECS + A-Tasks

- [Round up for the batch size to improve par_iter performance][9814]

### A-Reflection + A-Utils

- [Moved `fq_std` from `bevy_reflect_derive` to `bevy_macro_utils`][9956]

### A-Reflection + A-Math

- [Add reflect impls to IRect and URect][9191]
- [Implement reflect trait on new glam types (I64Vec and U64Vec)][9281]

### A-Hierarchy

- [Prevent setting parent as itself][8980]
- [Add as_slice to parent][9871]

### A-Input

- [input: allow multiple gamepad inputs to be registered for one button in one frame][9446]
- [Bevy Input Docs : lib.rs][9468]
- [Bevy Input Docs : gamepad.rs][9469]
- [Add `GamepadButtonInput` event][9008]
- [Bevy Input Docs : the modules][9467]
- [Finish documenting `bevy_gilrs`][10010]
- [Change `AxisSettings` livezone default][10090]
- [docs: Update input_toggle_active example][9913]

### A-Input + A-Windowing

- [Fix `Window::set_cursor_position`][9456]
- [Change `Window::physical_cursor_position` to use the physical size of the window][9657]
- [Fix check that cursor position is within window bounds][9662]

### A-ECS + A-Reflection

- [implement insert and remove reflected entity commands][8895]
- [Allow disjoint mutable world access via `EntityMut`][9419]
- [Implement `Reflect` for `State<S>` and `NextState<S>`][9742]
- [`#[derive(Clone)]` on `Component{Info,Descriptor}`][9812]

### A-Math

- [Rename bevy_math::rects conversion methods][9159]
- [Add glam swizzles traits to prelude][9387]
- [Rename `Bezier` to `CubicBezier` for clarity][9554]
- [Add a method to compute a bounding box enclosing a set of points][9630]
- [re-export `debug_glam_assert` feature][10206]
- [Add `Cubic` prefix to all cubic curve generators][10299]

### A-Build-System

- [only check for bans if the dependency tree changed][9252]
- [Slightly better message when contributor modifies examples template][9372]
- [switch CI jobs between windows and linux for example execution][9489]
- [Check for bevy_internal imports in CI][9612]
- [Fix running examples on linux in CI][9665]
- [Bump actions/checkout from 2 to 4][9759]
- [doc: Remove reference to `clippy::manual-strip`.][9794]
- [Only run some workflows on the bevy repo (not forks)][9872]
- [run mobile tests on more devices / OS versions][9936]
- [Allow `clippy::type_complexity` in more places.][9796]
- [hacks for running (and screenshotting) the examples in CI on a github runner][9220]
- [make CI less failing on cargo deny bans][10151]
- [add test on Android 14 / Pixel 8][10148]
- [Use `clippy::doc_markdown` more.][10286]

### A-Diagnostics

- [Cache System Tracing Spans][9390]

### A-Rendering + A-Animation

- [Use a seeded rng for custom_skinned_mesh example][9846]
- [Move skin code to a separate module][9899]

### A-Core

- [Change visibility of `bevy::core::update_frame_count` to `pub`][10111]

### A-Reflection

- [Fix typo in NamedTypePathDef][9102]
- [Refactor `path` module of `bevy_reflect`][8887]
- [Refactor parsing in bevy_reflect path module][9048]
- [bevy_reflect: Fix combined field attributes][9322]
- [bevy_reflect: Opt-out attribute for `TypePath`][9140]
- [Add reflect path parsing benchmark][9364]
- [Make it so `ParsedPath` can be passed to GetPath][9373]
- [Make the reflect path parser utf-8-unaware][9371]
- [bevy_scene: Add `ReflectBundle`][9165]
- [Fix comment in scene example `FromResources`][9743]
- [Remove TypeRegistry re-export rename][9807]
- [Provide getters for fields of ReflectFromPtr][9748]
- [Add TypePath to the prelude][9963]
- [Improve TypeUuid's derive macro error messages][9315]
- [Migrate `Quat` reflection strategy from "value" to "struct"][10068]
- [bevy_reflect: Fix dynamic type serialization][10103]
- [bevy_reflect: Fix ignored/skipped field order][7575]

### A-Rendering + A-Assets + A-Reflection

- [Implement `Reflect` for `Mesh`][9779]

### A-ECS + A-Time

- [add on_real_time_timer run condition][10179]

### A-ECS + A-Hierarchy

- [Added 'clear_children' and 'replace_children' methods to BuildWorldChildren to be consistent with BuildChildren.][10311]

### A-Audio

- [Added Pitch as an alternative sound source][9225]
- [update documentation on AudioSink][9332]
- [audio sinks don't need their custom drop anymore][9336]
- [Clarify what happens when setting the audio volume][9480]
- [More ergonomic spatial audio][9800]

### A-Rendering + A-UI

- [Remove out-of-date paragraph in `Style::border`][9103]
- [Revert "Fix UI corruption for AMD gpus with Vulkan (#9169)"][9237]
- [Revert "Fix UI corruption for AMD gpus with Vulkan (#9169)"][9237]
- [`many_buttons` enhancements][9712]
- [Fix UI borders][10078]
- [UI batching Fix][9610]
- [Add UI Materials][9506]

### A-ECS + A-Reflection + A-Pointers

- [add `MutUntyped::map_unchanged`][9194]

### No area label

- [Fix typos throughout the project][9090]
- [Bump Version after Release][9106]
- [fix `clippy::default_constructed_unit_structs` and trybuild errors][9144]
- [delete code deprecated in 0.11][9128]
- [Drain `ExtractedUiNodes` in `prepare_uinodes`][9142]
- [example showcase - pagination and can build for WebGL2][9168]
- [example showcase: switch default api to webgpu][9193]
- [Add some more helpful errors to BevyManifest when it doesn't find Cargo.toml][9207]
- [Fix path reference to contributors example][9219]
- [replace parens with square brackets when referencing _mut on `Query` docs #9200][9223]
- [use AutoNoVsync in stress tests][9229]
- [bevy_render: Remove direct dep on wgpu-hal.][9249]
- [Fixed typo in line 322][9276]
- [custom_material.vert: gl_InstanceIndex includes gl_BaseInstance][9326]
- [fix typo in a link - Mesh docs][9329]
- [Improve font size related docs][9320]
- [Fix gamepad viewer being marked as a non-wasm example][9399]
- [Rustdoc: Scrape examples][9154]
- [enable multithreading on benches][9388]
- [webgl feature renamed to webgl2][9370]
- [Example Comment Typo Fix][9427]
- [Fix shader_instancing example][9448]
- [Update tracy-client requirement from 0.15 to 0.16][9436]
- [fix bevy imports. windows_settings.rs example][9547]
- [Fix CI for Rust 1.72][9562]
- [Swap TransparentUi to use a stable sort][9598]
- [Replace uses of `entity.insert` with tuple bundles in `game_menu` example][9619]
- [Remove `IntoIterator` impl for `&mut EventReader`][9583]
- [remove VecSwizzles imports][9629]
- [Fix erronenous glam version][9653]
- [Fixing some doc comments][9646]
- [Explicitly make instance_index vertex output @interpolate(flat)][9675]
- [Fix some nightly warnings][9672]
- [Use default resolution for viewport_debug example][9666]
- [Refer to "macOS", not "macOS X".][9704]
- [Remove useless single tuples and trailing commas][9720]
- [Fix some warnings shown in nightly][10012]
- [Fix animate_scale scaling z value in text2d example][9769]
- ["serialize" feature no longer enables the optional "bevy_scene" feature if it's not enabled from elsewhere][9803]
- [fix deprecation warning in bench][9823]
- [don't enable filesystem_watcher when building for WebGPU][9829]
- [Improve doc formatting.][9840]
- [Fix the `clippy::explicit_iter_loop` lint][9834]
- [Wslg docs][9842]
- [skybox.wgsl: Fix precision issues][9909]
- [Fix typos.][9922]
- [Add link to `Text2dBundle` in `TextBundle` docs.][9900]
- [Fix some typos][9934]
- [Fix typos][9965]
- [Replaced `parking_lot` with `std::sync`][9545]
- [Add inline(never) to bench systems][9824]
- [Android: handle suspend / resume][9937]
- [Fix some warnings shown in nightly][10012]
- [Updates for rust 1.73][10035]
- [Improve selection of iOS device in mobile example][9282]
- [Update toml_edit requirement from 0.19 to 0.20][10058]
- [foxes shouldn't march in sync][10070]
- [Fix tonemapping test patten][10092]
- [Removed `once_cell`][10079]
- [Improve WebGPU unstable flags docs][10163]
- [shadow_biases: Support different PCF methods][10184]
- [shadow_biases: Support moving the light position and resetting biases][10185]
- [Update async-io requirement from 1.13.0 to 2.0.0][10238]
- [few fmt tweaks][10264]
- [Derive Error for more error types][10240]
- [Allow AccessKit to react to WindowEvents before they reach the engine][10356]

### A-Rendering + A-Build-System

- [Improve execution of examples in CI][9331]
- [make deferred_rendering simpler to render for CI][10150]

### A-Meta

- [Remove the bevy_dylib feature][9516]
- [add and fix shields in Readmes][9993]
- [Added section for contributing and links for issues and PRs][10171]
- [Fix orphaned contributing paragraph][10174]

### A-Assets + A-Animation

- [Handle empty morph weights when loading gltf][9867]
- [Finish documenting `bevy_gltf`][9998]

### A-Editor + A-Diagnostics

- [Add `DiagnosticsStore::iter_mut`][9679]

### A-Time

- [Fix timers.rs documentation][9290]
- [Add missing documentation to `bevy_time`][9428]
- [Clarify behaviour of `Timer::finished()` for repeating timers][9939]
- [ignore time channel error][9981]
- [Unify `FixedTime` and `Time` while fixing several problems][8964]
- [Time: demote delta time clamping warning to debug][10145]
- [fix typo in time.rs example][10152]
- [Example time api][10204]

### A-Rendering + A-ECS

- [Update `Camera`'s `Frustum` only when its `GlobalTransform` or `CameraProjection` changed][9092]

### A-UI + A-Reflection

- [bevy_ui: reflect missing types][9677]
- [register `TextLayoutInfo` and `TextFlags` type.][9919]

### A-Build-System + A-Assets

- [Increase iteration count for asset tests][9737]

### A-Rendering

- [Clarify that wgpu is based on the webGPU API][9093]
- [Return URect instead of (UVec2, UVec2) in Camera::physical_viewport_rect][9085]
- [fix module name for AssetPath shaders][9186]
- [Add GpuArrayBuffer and BatchedUniformBuffer][8204]
- [Update `bevy_window::PresentMode` to mirror `wgpu::PresentMode`][9230]
- [Stop using unwrap in the pipelined rendering thread][9052]
- [Fix panic whilst loading UASTC encoded ktx2 textures][9158]
- [Document `ClearColorConfig`][9288]
- [Use GpuArrayBuffer for MeshUniform][9254]
- [Update docs for scaling_mode field of Orthographic projection][9297]
- [Fix shader_material_glsl example after #9254][9311]
- [Improve `Mesh` documentation][9061]
- [Include tone_mapping fn in tonemapping_test_patterns][9084]
- [Extend the default render range of 2D camera][9310]
- [Document when Camera::viewport_to_world and related methods return None][8841]
- [include toplevel shader-associated defs][9343]
- [Fix post_processing example on webgl2][9361]
- [use ViewNodeRunner in the post_processing example][9127]
- [Work around naga/wgpu WGSL instance_index -> GLSL gl_InstanceID bug on WebGL2][9383]
- [Fix non-visible motion vector text in shader prepass example][9155]
- [Use bevy crates imports instead of bevy internal. post_processing example][9396]
- [Make Anchor Copy][9327]
- [Move window.rs to window/mod.rs in bevy_render][9394]
- [Reduce the size of MeshUniform to improve performance][9416]
- [Fix temporal jitter bug][9462]
- [Fix gizmo lines deforming or disappearing when partially behind the camera][9470]
- [Make WgpuSettings::default() check WGPU_POWER_PREF][9482]
- [fix wireframe after MeshUniform size reduction][9505]
- [fix shader_material_glsl example][9513]
- [[RAINBOW EFFECT] Added methods to get HSL components from Color][9201]
- [ktx2: Fix Rgb8 -> Rgba8Unorm conversion][9555]
- [Reorder render sets, refactor bevy_sprite to take advantage][9236]
- [Improve documentation relating to `Frustum` and `HalfSpace`][9136]
- [Revert "Update defaults for OrthographicProjection (#9537)"][9878]
- [Remove unused regex dep from bevy_render][9613]
- [Split `ComputedVisibility` into two components to allow for accurate change detection and speed up visibility propagation][9497]
- [Use instancing for sprites][9597]
- [Enhance bevymark][9674]
- [Remove redundant math in tonemapping.][9669]
- [Improve `SpatialBundle` docs][9673]
- [Cache depth texture based on usage][9565]
- [warn and min for different vertex count][9699]
- [default 16bit rgb/rgba textures to unorm instead of uint][9611]
- [Fix TextureAtlasBuilder padding][10031]
- [Add example for `Camera::viewport_to_world`][7179]
- [Fix wireframe for skinned/morphed meshes][9734]
- [generate indices for Mikktspace][8862]
- [invert face culling for negatively scaled gltf nodes][8859]
- [renderer init: create a detached task only on wasm, block otherwise][9830]
- [Cleanup `visibility` module][9850]
- [Use a single line for of large binding lists][9849]
- [Fix a typo in `DirectionalLightBundle`][9861]
- [Revert "Update defaults for OrthographicProjection (#9537)"][9878]
- [Refactor rendering systems to use `let-else`][9870]
- [Use radsort for Transparent2d PhaseItem sorting][9882]
- [Automatic batching/instancing of draw commands][9685]
- [Directly copy data into uniform buffers][9865]
- [Allow other plugins to create renderer resources][9925]
- [Use EntityHashMap<Entity, T> for render world entity storage for better performance][9903]
- [Parallelize extract_meshes][9966]
- [Fix comment grammar][9990]
- [Allow overriding global wireframe setting.][7328]
- [wireframes: workaround for DX12][10022]
- [Alternate wireframe override api][10023]
- [Fix TextureAtlasBuilder padding][10031]
- [fix example mesh2d_manual][9941]
- [PCF For DirectionalLight/SpotLight Shadows][8006]
- [Refactor the render instance logic in #9903 so that it's easier for other components to adopt.][10002]
- [Fix 2d_shapes and general 2D mesh instancing][10051]
- [fix webgl2 crash][10053]
- [fix orthographic cluster aabb for spotlight culling][9614]
- [Add consuming builder methods for more ergonomic `Mesh` creation][10056]
- [wgpu 0.17][9302]
- [use `Material` for wireframes][5314]
- [Extract common wireframe filters in type alias][10080]
- [Deferred Renderer][9258]
- [Configurable colors for wireframe][5303]
- [chore: Renamed RenderInstance trait to ExtractInstance][10065]
- [pbr shader cleanup][10105]
- [Fix text2d view-visibility][10100]
- [Allow optional extraction of resources from the main world][10109]
- [ssao use unlit_color instead of white][10117]
- [Fix missing explicit lifetime name for copy_deferred_lighting_id name][10128]
- [Fixed mod.rs in rendering to support Radeon Cards][10132]
- [Explain usage of prepass shaders in docs for `Material` trait][9025]
- [Better link for prepare_windows docs][10142]
- [Improve linking within `RenderSet` docs.][10143]
- [Fix unlit missing parameters][10144]
- [`*_PREPASS` Shader Def Cleanup][10136]
- [check for any prepass phase][10160]
- [allow extensions to StandardMaterial][7820]
- [array_texture example: use new name of pbr function][10168]
- [chore: use ExtractComponent derive macro for EnvironmentMapLight and FogSettings][10191]
- [Variable `MeshPipeline` View Bind Group Layout][10156]
- [update shader imports][10180]
- [Bind group entries][9694]
- [Detect cubemap for dds textures][10222]
- [Fix alignment on ios simulator][10178]
- [Add convenient methods for Image][10221]
- [Use “specular occlusion” term to consistently extinguish fresnel on Ambient and Environment Map lights][10182]
- [Fix fog color being inaccurate][10226]
- [Replace all usages of texture_descritor.size.* with the helper methods][10227]
- [View Transformations][9726]
- [fix deferred example fog values][10249]
- [WebGL2: fix import path for unpack_unorm3x4_plus_unorm_20_][10251]
- [Use wildcard imports in bevy_pbr][9847]
- [Make mesh attr vertex count mismatch warn more readable][10259]
- [Image Sampler Improvements][10254]
- [Fix sampling of diffuse env map texture with non-uniform control flow][10276]
- [Log a warning when the `tonemapping_luts` feature is disabled but required for the selected tonemapper.][10253]
- [Smaller TAA fixes][10200]
- [Truncate attribute buffer data rather than attribute buffers][10270]
- [Fix deferred lighting pass values not all working on M1 in WebGL2][10304]
- [Add frustum to shader View][10306]
- [Fix handling of `double_sided` for normal maps][10326]
- [Add helper function to determine if color is transparent][10310]
- [`StandardMaterial` Light Transmission][8015]
- [double sided normals: fix apply_normal_mapping calls][10330]
- [Combine visibility queries in check_visibility_system][10196]
- [Make VERTEX_COLORS usable in prepass shader, if available][10341]
- [allow DeferredPrepass to work without other prepass markers][10223]
- [Increase default normal bias to avoid common artifacts][10346]
- [Make `DirectionalLight` `Cascades` computation generic over `CameraProjection`][9226]
- [Update default `ClearColor` to better match Bevy's branding][10339]
- [Fix gizmo crash when prepass enabled][10360]

### A-Build-System + A-Meta

- [Fixed: README.md][9994]

### A-Assets

- [doc(asset): fix asset trait example][9105]
- [Add `GltfLoader::new`.][9120]
- [impl `From<&AssetPath>` for `HandleId`][9132]
- [allow asset loader pre-registration][9429]
- [fix asset loader preregistration for multiple assets][9453]
- [Fix point light radius][9493]
- [Add support for KHR_materials_emissive_strength][9553]
- [Fix panic when using `.load_folder()` with absolute paths][9490]
- [Bevy Asset V2][8624]
- [create imported asset directory if needed][9716]
- [Copy on Write AssetPaths][9729]
- [Asset v2: Asset path serialization fix][9756]
- [don't ignore some EventKind::Modify][9767]
- [Manual "Reflect Value" AssetPath impl to fix dynamic linking][9752]
- [Fix unused variable warning for simple AssetV2 derives][9961]
- [Remove monkey.gltf][9974]
- [Update notify-debouncer-full requirement from 0.2.0 to 0.3.1][9757]
- [Removed `anyhow`][10003]
- [Multiple Asset Sources][9885]
- [Make loading warning for no file ext more descriptive][10119]
- [Fix load_folder for non-default Asset Sources][10121]
- [only set up processed source if asset plugin is not unprocessed][10123]
- [Hot reload labeled assets whose source asset is not loaded][9736]
- [Return an error when loading non-existent labels][9751]
- [remove unused import on android][10197]
- [Log an error when registering an AssetSource after AssetPlugin has been built][10202]
- [Add note about asset source register order][10186]
- [Add `asset_processor` feature and remove AssetMode::ProcessedDev][10194]
- [Implement source into Display for AssetPath][10217]
- [assets: use blake3 instead of md5][10208]
- [Reduce noise in asset processing example][10262]
- [Adding AssetPath::resolve() method.][9528]
- [Assets: fix first hot reloading][9804]
- [Non-blocking load_untyped using a wrapper asset][10198]
- [Reuse and hot reload folder handles][10210]
- [Additional AssetPath unit tests.][10279]
- [Corrected incorrect doc comment on read_asset_bytes][10352]
- [support file operations in single threaded context][10312]

[5303]: https://github.com/bevyengine/bevy/pull/5303
[5314]: https://github.com/bevyengine/bevy/pull/5314
[7179]: https://github.com/bevyengine/bevy/pull/7179
[7328]: https://github.com/bevyengine/bevy/pull/7328
[7575]: https://github.com/bevyengine/bevy/pull/7575
[7609]: https://github.com/bevyengine/bevy/pull/7609
[7611]: https://github.com/bevyengine/bevy/pull/7611
[7728]: https://github.com/bevyengine/bevy/pull/7728
[7762]: https://github.com/bevyengine/bevy/pull/7762
[7794]: https://github.com/bevyengine/bevy/pull/7794
[7820]: https://github.com/bevyengine/bevy/pull/7820
[8006]: https://github.com/bevyengine/bevy/pull/8006
[8015]: https://github.com/bevyengine/bevy/pull/8015
[8204]: https://github.com/bevyengine/bevy/pull/8204
[8246]: https://github.com/bevyengine/bevy/pull/8246
[8526]: https://github.com/bevyengine/bevy/pull/8526
[8603]: https://github.com/bevyengine/bevy/pull/8603
[8624]: https://github.com/bevyengine/bevy/pull/8624
[8750]: https://github.com/bevyengine/bevy/pull/8750
[8768]: https://github.com/bevyengine/bevy/pull/8768
[8841]: https://github.com/bevyengine/bevy/pull/8841
[8854]: https://github.com/bevyengine/bevy/pull/8854
[8855]: https://github.com/bevyengine/bevy/pull/8855
[8856]: https://github.com/bevyengine/bevy/pull/8856
[8859]: https://github.com/bevyengine/bevy/pull/8859
[8862]: https://github.com/bevyengine/bevy/pull/8862
[8887]: https://github.com/bevyengine/bevy/pull/8887
[8895]: https://github.com/bevyengine/bevy/pull/8895
[8963]: https://github.com/bevyengine/bevy/pull/8963
[8964]: https://github.com/bevyengine/bevy/pull/8964
[8980]: https://github.com/bevyengine/bevy/pull/8980
[9002]: https://github.com/bevyengine/bevy/pull/9002
[9008]: https://github.com/bevyengine/bevy/pull/9008
[9025]: https://github.com/bevyengine/bevy/pull/9025
[9048]: https://github.com/bevyengine/bevy/pull/9048
[9052]: https://github.com/bevyengine/bevy/pull/9052
[9061]: https://github.com/bevyengine/bevy/pull/9061
[9067]: https://github.com/bevyengine/bevy/pull/9067
[9068]: https://github.com/bevyengine/bevy/pull/9068
[9081]: https://github.com/bevyengine/bevy/pull/9081
[9083]: https://github.com/bevyengine/bevy/pull/9083
[9084]: https://github.com/bevyengine/bevy/pull/9084
[9085]: https://github.com/bevyengine/bevy/pull/9085
[9090]: https://github.com/bevyengine/bevy/pull/9090
[9092]: https://github.com/bevyengine/bevy/pull/9092
[9093]: https://github.com/bevyengine/bevy/pull/9093
[9099]: https://github.com/bevyengine/bevy/pull/9099
[9101]: https://github.com/bevyengine/bevy/pull/9101
[9102]: https://github.com/bevyengine/bevy/pull/9102
[9103]: https://github.com/bevyengine/bevy/pull/9103
[9105]: https://github.com/bevyengine/bevy/pull/9105
[9106]: https://github.com/bevyengine/bevy/pull/9106
[9111]: https://github.com/bevyengine/bevy/pull/9111
[9120]: https://github.com/bevyengine/bevy/pull/9120
[9127]: https://github.com/bevyengine/bevy/pull/9127
[9128]: https://github.com/bevyengine/bevy/pull/9128
[9129]: https://github.com/bevyengine/bevy/pull/9129
[9132]: https://github.com/bevyengine/bevy/pull/9132
[9133]: https://github.com/bevyengine/bevy/pull/9133
[9136]: https://github.com/bevyengine/bevy/pull/9136
[9140]: https://github.com/bevyengine/bevy/pull/9140
[9142]: https://github.com/bevyengine/bevy/pull/9142
[9144]: https://github.com/bevyengine/bevy/pull/9144
[9154]: https://github.com/bevyengine/bevy/pull/9154
[9155]: https://github.com/bevyengine/bevy/pull/9155
[9158]: https://github.com/bevyengine/bevy/pull/9158
[9159]: https://github.com/bevyengine/bevy/pull/9159
[9162]: https://github.com/bevyengine/bevy/pull/9162
[9165]: https://github.com/bevyengine/bevy/pull/9165
[9166]: https://github.com/bevyengine/bevy/pull/9166
[9168]: https://github.com/bevyengine/bevy/pull/9168
[9173]: https://github.com/bevyengine/bevy/pull/9173
[9174]: https://github.com/bevyengine/bevy/pull/9174
[9175]: https://github.com/bevyengine/bevy/pull/9175
[9183]: https://github.com/bevyengine/bevy/pull/9183
[9186]: https://github.com/bevyengine/bevy/pull/9186
[9191]: https://github.com/bevyengine/bevy/pull/9191
[9193]: https://github.com/bevyengine/bevy/pull/9193
[9194]: https://github.com/bevyengine/bevy/pull/9194
[9201]: https://github.com/bevyengine/bevy/pull/9201
[9204]: https://github.com/bevyengine/bevy/pull/9204
[9207]: https://github.com/bevyengine/bevy/pull/9207
[9219]: https://github.com/bevyengine/bevy/pull/9219
[9220]: https://github.com/bevyengine/bevy/pull/9220
[9222]: https://github.com/bevyengine/bevy/pull/9222
[9223]: https://github.com/bevyengine/bevy/pull/9223
[9225]: https://github.com/bevyengine/bevy/pull/9225
[9226]: https://github.com/bevyengine/bevy/pull/9226
[9229]: https://github.com/bevyengine/bevy/pull/9229
[9230]: https://github.com/bevyengine/bevy/pull/9230
[9236]: https://github.com/bevyengine/bevy/pull/9236
[9237]: https://github.com/bevyengine/bevy/pull/9237
[9247]: https://github.com/bevyengine/bevy/pull/9247
[9249]: https://github.com/bevyengine/bevy/pull/9249
[9252]: https://github.com/bevyengine/bevy/pull/9252
[9253]: https://github.com/bevyengine/bevy/pull/9253
[9254]: https://github.com/bevyengine/bevy/pull/9254
[9258]: https://github.com/bevyengine/bevy/pull/9258
[9259]: https://github.com/bevyengine/bevy/pull/9259
[9260]: https://github.com/bevyengine/bevy/pull/9260
[9269]: https://github.com/bevyengine/bevy/pull/9269
[9276]: https://github.com/bevyengine/bevy/pull/9276
[9281]: https://github.com/bevyengine/bevy/pull/9281
[9282]: https://github.com/bevyengine/bevy/pull/9282
[9285]: https://github.com/bevyengine/bevy/pull/9285
[9288]: https://github.com/bevyengine/bevy/pull/9288
[9290]: https://github.com/bevyengine/bevy/pull/9290
[9291]: https://github.com/bevyengine/bevy/pull/9291
[9293]: https://github.com/bevyengine/bevy/pull/9293
[9295]: https://github.com/bevyengine/bevy/pull/9295
[9297]: https://github.com/bevyengine/bevy/pull/9297
[9301]: https://github.com/bevyengine/bevy/pull/9301
[9302]: https://github.com/bevyengine/bevy/pull/9302
[9306]: https://github.com/bevyengine/bevy/pull/9306
[9310]: https://github.com/bevyengine/bevy/pull/9310
[9311]: https://github.com/bevyengine/bevy/pull/9311
[9312]: https://github.com/bevyengine/bevy/pull/9312
[9313]: https://github.com/bevyengine/bevy/pull/9313
[9315]: https://github.com/bevyengine/bevy/pull/9315
[9320]: https://github.com/bevyengine/bevy/pull/9320
[9322]: https://github.com/bevyengine/bevy/pull/9322
[9326]: https://github.com/bevyengine/bevy/pull/9326
[9327]: https://github.com/bevyengine/bevy/pull/9327
[9329]: https://github.com/bevyengine/bevy/pull/9329
[9331]: https://github.com/bevyengine/bevy/pull/9331
[9332]: https://github.com/bevyengine/bevy/pull/9332
[9336]: https://github.com/bevyengine/bevy/pull/9336
[9338]: https://github.com/bevyengine/bevy/pull/9338
[9343]: https://github.com/bevyengine/bevy/pull/9343
[9344]: https://github.com/bevyengine/bevy/pull/9344
[9345]: https://github.com/bevyengine/bevy/pull/9345
[9346]: https://github.com/bevyengine/bevy/pull/9346
[9348]: https://github.com/bevyengine/bevy/pull/9348
[9351]: https://github.com/bevyengine/bevy/pull/9351
[9355]: https://github.com/bevyengine/bevy/pull/9355
[9360]: https://github.com/bevyengine/bevy/pull/9360
[9361]: https://github.com/bevyengine/bevy/pull/9361
[9364]: https://github.com/bevyengine/bevy/pull/9364
[9366]: https://github.com/bevyengine/bevy/pull/9366
[9370]: https://github.com/bevyengine/bevy/pull/9370
[9371]: https://github.com/bevyengine/bevy/pull/9371
[9372]: https://github.com/bevyengine/bevy/pull/9372
[9373]: https://github.com/bevyengine/bevy/pull/9373
[9383]: https://github.com/bevyengine/bevy/pull/9383
[9385]: https://github.com/bevyengine/bevy/pull/9385
[9387]: https://github.com/bevyengine/bevy/pull/9387
[9388]: https://github.com/bevyengine/bevy/pull/9388
[9390]: https://github.com/bevyengine/bevy/pull/9390
[9392]: https://github.com/bevyengine/bevy/pull/9392
[9394]: https://github.com/bevyengine/bevy/pull/9394
[9396]: https://github.com/bevyengine/bevy/pull/9396
[9399]: https://github.com/bevyengine/bevy/pull/9399
[9404]: https://github.com/bevyengine/bevy/pull/9404
[9406]: https://github.com/bevyengine/bevy/pull/9406
[9407]: https://github.com/bevyengine/bevy/pull/9407
[9416]: https://github.com/bevyengine/bevy/pull/9416
[9418]: https://github.com/bevyengine/bevy/pull/9418
[9419]: https://github.com/bevyengine/bevy/pull/9419
[9421]: https://github.com/bevyengine/bevy/pull/9421
[9427]: https://github.com/bevyengine/bevy/pull/9427
[9428]: https://github.com/bevyengine/bevy/pull/9428
[9429]: https://github.com/bevyengine/bevy/pull/9429
[9431]: https://github.com/bevyengine/bevy/pull/9431
[9436]: https://github.com/bevyengine/bevy/pull/9436
[9444]: https://github.com/bevyengine/bevy/pull/9444
[9446]: https://github.com/bevyengine/bevy/pull/9446
[9448]: https://github.com/bevyengine/bevy/pull/9448
[9451]: https://github.com/bevyengine/bevy/pull/9451
[9453]: https://github.com/bevyengine/bevy/pull/9453
[9456]: https://github.com/bevyengine/bevy/pull/9456
[9460]: https://github.com/bevyengine/bevy/pull/9460
[9461]: https://github.com/bevyengine/bevy/pull/9461
[9462]: https://github.com/bevyengine/bevy/pull/9462
[9463]: https://github.com/bevyengine/bevy/pull/9463
[9464]: https://github.com/bevyengine/bevy/pull/9464
[9467]: https://github.com/bevyengine/bevy/pull/9467
[9468]: https://github.com/bevyengine/bevy/pull/9468
[9469]: https://github.com/bevyengine/bevy/pull/9469
[9470]: https://github.com/bevyengine/bevy/pull/9470
[9479]: https://github.com/bevyengine/bevy/pull/9479
[9480]: https://github.com/bevyengine/bevy/pull/9480
[9481]: https://github.com/bevyengine/bevy/pull/9481
[9482]: https://github.com/bevyengine/bevy/pull/9482
[9484]: https://github.com/bevyengine/bevy/pull/9484
[9485]: https://github.com/bevyengine/bevy/pull/9485
[9486]: https://github.com/bevyengine/bevy/pull/9486
[9489]: https://github.com/bevyengine/bevy/pull/9489
[9490]: https://github.com/bevyengine/bevy/pull/9490
[9493]: https://github.com/bevyengine/bevy/pull/9493
[9497]: https://github.com/bevyengine/bevy/pull/9497
[9505]: https://github.com/bevyengine/bevy/pull/9505
[9506]: https://github.com/bevyengine/bevy/pull/9506
[9513]: https://github.com/bevyengine/bevy/pull/9513
[9514]: https://github.com/bevyengine/bevy/pull/9514
[9516]: https://github.com/bevyengine/bevy/pull/9516
[9518]: https://github.com/bevyengine/bevy/pull/9518
[9528]: https://github.com/bevyengine/bevy/pull/9528
[9529]: https://github.com/bevyengine/bevy/pull/9529
[9539]: https://github.com/bevyengine/bevy/pull/9539
[9542]: https://github.com/bevyengine/bevy/pull/9542
[9545]: https://github.com/bevyengine/bevy/pull/9545
[9547]: https://github.com/bevyengine/bevy/pull/9547
[9548]: https://github.com/bevyengine/bevy/pull/9548
[9551]: https://github.com/bevyengine/bevy/pull/9551
[9553]: https://github.com/bevyengine/bevy/pull/9553
[9554]: https://github.com/bevyengine/bevy/pull/9554
[9555]: https://github.com/bevyengine/bevy/pull/9555
[9562]: https://github.com/bevyengine/bevy/pull/9562
[9565]: https://github.com/bevyengine/bevy/pull/9565
[9566]: https://github.com/bevyengine/bevy/pull/9566
[9568]: https://github.com/bevyengine/bevy/pull/9568
[9576]: https://github.com/bevyengine/bevy/pull/9576
[9577]: https://github.com/bevyengine/bevy/pull/9577
[9579]: https://github.com/bevyengine/bevy/pull/9579
[9581]: https://github.com/bevyengine/bevy/pull/9581
[9583]: https://github.com/bevyengine/bevy/pull/9583
[9587]: https://github.com/bevyengine/bevy/pull/9587
[9588]: https://github.com/bevyengine/bevy/pull/9588
[9589]: https://github.com/bevyengine/bevy/pull/9589
[9592]: https://github.com/bevyengine/bevy/pull/9592
[9596]: https://github.com/bevyengine/bevy/pull/9596
[9597]: https://github.com/bevyengine/bevy/pull/9597
[9598]: https://github.com/bevyengine/bevy/pull/9598
[9600]: https://github.com/bevyengine/bevy/pull/9600
[9604]: https://github.com/bevyengine/bevy/pull/9604
[9609]: https://github.com/bevyengine/bevy/pull/9609
[9610]: https://github.com/bevyengine/bevy/pull/9610
[9611]: https://github.com/bevyengine/bevy/pull/9611
[9612]: https://github.com/bevyengine/bevy/pull/9612
[9613]: https://github.com/bevyengine/bevy/pull/9613
[9614]: https://github.com/bevyengine/bevy/pull/9614
[9617]: https://github.com/bevyengine/bevy/pull/9617
[9619]: https://github.com/bevyengine/bevy/pull/9619
[9621]: https://github.com/bevyengine/bevy/pull/9621
[9626]: https://github.com/bevyengine/bevy/pull/9626
[9629]: https://github.com/bevyengine/bevy/pull/9629
[9630]: https://github.com/bevyengine/bevy/pull/9630
[9631]: https://github.com/bevyengine/bevy/pull/9631
[9637]: https://github.com/bevyengine/bevy/pull/9637
[9638]: https://github.com/bevyengine/bevy/pull/9638
[9640]: https://github.com/bevyengine/bevy/pull/9640
[9646]: https://github.com/bevyengine/bevy/pull/9646
[9648]: https://github.com/bevyengine/bevy/pull/9648
[9650]: https://github.com/bevyengine/bevy/pull/9650
[9653]: https://github.com/bevyengine/bevy/pull/9653
[9657]: https://github.com/bevyengine/bevy/pull/9657
[9659]: https://github.com/bevyengine/bevy/pull/9659
[9662]: https://github.com/bevyengine/bevy/pull/9662
[9665]: https://github.com/bevyengine/bevy/pull/9665
[9666]: https://github.com/bevyengine/bevy/pull/9666
[9669]: https://github.com/bevyengine/bevy/pull/9669
[9672]: https://github.com/bevyengine/bevy/pull/9672
[9673]: https://github.com/bevyengine/bevy/pull/9673
[9674]: https://github.com/bevyengine/bevy/pull/9674
[9675]: https://github.com/bevyengine/bevy/pull/9675
[9677]: https://github.com/bevyengine/bevy/pull/9677
[9679]: https://github.com/bevyengine/bevy/pull/9679
[9684]: https://github.com/bevyengine/bevy/pull/9684
[9685]: https://github.com/bevyengine/bevy/pull/9685
[9686]: https://github.com/bevyengine/bevy/pull/9686
[9692]: https://github.com/bevyengine/bevy/pull/9692
[9694]: https://github.com/bevyengine/bevy/pull/9694
[9697]: https://github.com/bevyengine/bevy/pull/9697
[9699]: https://github.com/bevyengine/bevy/pull/9699
[9704]: https://github.com/bevyengine/bevy/pull/9704
[9705]: https://github.com/bevyengine/bevy/pull/9705
[9712]: https://github.com/bevyengine/bevy/pull/9712
[9716]: https://github.com/bevyengine/bevy/pull/9716
[9720]: https://github.com/bevyengine/bevy/pull/9720
[9726]: https://github.com/bevyengine/bevy/pull/9726
[9729]: https://github.com/bevyengine/bevy/pull/9729
[9733]: https://github.com/bevyengine/bevy/pull/9733
[9734]: https://github.com/bevyengine/bevy/pull/9734
[9736]: https://github.com/bevyengine/bevy/pull/9736
[9737]: https://github.com/bevyengine/bevy/pull/9737
[9739]: https://github.com/bevyengine/bevy/pull/9739
[9741]: https://github.com/bevyengine/bevy/pull/9741
[9742]: https://github.com/bevyengine/bevy/pull/9742
[9743]: https://github.com/bevyengine/bevy/pull/9743
[9744]: https://github.com/bevyengine/bevy/pull/9744
[9748]: https://github.com/bevyengine/bevy/pull/9748
[9749]: https://github.com/bevyengine/bevy/pull/9749
[9751]: https://github.com/bevyengine/bevy/pull/9751
[9752]: https://github.com/bevyengine/bevy/pull/9752
[9753]: https://github.com/bevyengine/bevy/pull/9753
[9756]: https://github.com/bevyengine/bevy/pull/9756
[9757]: https://github.com/bevyengine/bevy/pull/9757
[9759]: https://github.com/bevyengine/bevy/pull/9759
[9767]: https://github.com/bevyengine/bevy/pull/9767
[9769]: https://github.com/bevyengine/bevy/pull/9769
[9778]: https://github.com/bevyengine/bevy/pull/9778
[9779]: https://github.com/bevyengine/bevy/pull/9779
[9781]: https://github.com/bevyengine/bevy/pull/9781
[9784]: https://github.com/bevyengine/bevy/pull/9784
[9785]: https://github.com/bevyengine/bevy/pull/9785
[9794]: https://github.com/bevyengine/bevy/pull/9794
[9795]: https://github.com/bevyengine/bevy/pull/9795
[9796]: https://github.com/bevyengine/bevy/pull/9796
[9800]: https://github.com/bevyengine/bevy/pull/9800
[9801]: https://github.com/bevyengine/bevy/pull/9801
[9803]: https://github.com/bevyengine/bevy/pull/9803
[9804]: https://github.com/bevyengine/bevy/pull/9804
[9807]: https://github.com/bevyengine/bevy/pull/9807
[9812]: https://github.com/bevyengine/bevy/pull/9812
[9814]: https://github.com/bevyengine/bevy/pull/9814
[9818]: https://github.com/bevyengine/bevy/pull/9818
[9820]: https://github.com/bevyengine/bevy/pull/9820
[9823]: https://github.com/bevyengine/bevy/pull/9823
[9824]: https://github.com/bevyengine/bevy/pull/9824
[9825]: https://github.com/bevyengine/bevy/pull/9825
[9826]: https://github.com/bevyengine/bevy/pull/9826
[9829]: https://github.com/bevyengine/bevy/pull/9829
[9830]: https://github.com/bevyengine/bevy/pull/9830
[9833]: https://github.com/bevyengine/bevy/pull/9833
[9834]: https://github.com/bevyengine/bevy/pull/9834
[9835]: https://github.com/bevyengine/bevy/pull/9835
[9836]: https://github.com/bevyengine/bevy/pull/9836
[9839]: https://github.com/bevyengine/bevy/pull/9839
[9840]: https://github.com/bevyengine/bevy/pull/9840
[9842]: https://github.com/bevyengine/bevy/pull/9842
[9844]: https://github.com/bevyengine/bevy/pull/9844
[9845]: https://github.com/bevyengine/bevy/pull/9845
[9846]: https://github.com/bevyengine/bevy/pull/9846
[9847]: https://github.com/bevyengine/bevy/pull/9847
[9849]: https://github.com/bevyengine/bevy/pull/9849
[9850]: https://github.com/bevyengine/bevy/pull/9850
[9851]: https://github.com/bevyengine/bevy/pull/9851
[9853]: https://github.com/bevyengine/bevy/pull/9853
[9861]: https://github.com/bevyengine/bevy/pull/9861
[9865]: https://github.com/bevyengine/bevy/pull/9865
[9867]: https://github.com/bevyengine/bevy/pull/9867
[9870]: https://github.com/bevyengine/bevy/pull/9870
[9871]: https://github.com/bevyengine/bevy/pull/9871
[9872]: https://github.com/bevyengine/bevy/pull/9872
[9873]: https://github.com/bevyengine/bevy/pull/9873
[9878]: https://github.com/bevyengine/bevy/pull/9878
[9879]: https://github.com/bevyengine/bevy/pull/9879
[9881]: https://github.com/bevyengine/bevy/pull/9881
[9882]: https://github.com/bevyengine/bevy/pull/9882
[9885]: https://github.com/bevyengine/bevy/pull/9885
[9895]: https://github.com/bevyengine/bevy/pull/9895
[9899]: https://github.com/bevyengine/bevy/pull/9899
[9900]: https://github.com/bevyengine/bevy/pull/9900
[9903]: https://github.com/bevyengine/bevy/pull/9903
[9909]: https://github.com/bevyengine/bevy/pull/9909
[9912]: https://github.com/bevyengine/bevy/pull/9912
[9913]: https://github.com/bevyengine/bevy/pull/9913
[9919]: https://github.com/bevyengine/bevy/pull/9919
[9922]: https://github.com/bevyengine/bevy/pull/9922
[9923]: https://github.com/bevyengine/bevy/pull/9923
[9925]: https://github.com/bevyengine/bevy/pull/9925
[9927]: https://github.com/bevyengine/bevy/pull/9927
[9931]: https://github.com/bevyengine/bevy/pull/9931
[9933]: https://github.com/bevyengine/bevy/pull/9933
[9934]: https://github.com/bevyengine/bevy/pull/9934
[9935]: https://github.com/bevyengine/bevy/pull/9935
[9936]: https://github.com/bevyengine/bevy/pull/9936
[9937]: https://github.com/bevyengine/bevy/pull/9937
[9939]: https://github.com/bevyengine/bevy/pull/9939
[9941]: https://github.com/bevyengine/bevy/pull/9941
[9945]: https://github.com/bevyengine/bevy/pull/9945
[9949]: https://github.com/bevyengine/bevy/pull/9949
[9950]: https://github.com/bevyengine/bevy/pull/9950
[9951]: https://github.com/bevyengine/bevy/pull/9951
[9956]: https://github.com/bevyengine/bevy/pull/9956
[9958]: https://github.com/bevyengine/bevy/pull/9958
[9959]: https://github.com/bevyengine/bevy/pull/9959
[9961]: https://github.com/bevyengine/bevy/pull/9961
[9963]: https://github.com/bevyengine/bevy/pull/9963
[9965]: https://github.com/bevyengine/bevy/pull/9965
[9966]: https://github.com/bevyengine/bevy/pull/9966
[9974]: https://github.com/bevyengine/bevy/pull/9974
[9981]: https://github.com/bevyengine/bevy/pull/9981
[9982]: https://github.com/bevyengine/bevy/pull/9982
[9984]: https://github.com/bevyengine/bevy/pull/9984
[9989]: https://github.com/bevyengine/bevy/pull/9989
[9990]: https://github.com/bevyengine/bevy/pull/9990
[9991]: https://github.com/bevyengine/bevy/pull/9991
[9992]: https://github.com/bevyengine/bevy/pull/9992
[9993]: https://github.com/bevyengine/bevy/pull/9993
[9994]: https://github.com/bevyengine/bevy/pull/9994
[9998]: https://github.com/bevyengine/bevy/pull/9998
[10001]: https://github.com/bevyengine/bevy/pull/10001
[10002]: https://github.com/bevyengine/bevy/pull/10002
[10003]: https://github.com/bevyengine/bevy/pull/10003
[10008]: https://github.com/bevyengine/bevy/pull/10008
[10010]: https://github.com/bevyengine/bevy/pull/10010
[10012]: https://github.com/bevyengine/bevy/pull/10012
[10020]: https://github.com/bevyengine/bevy/pull/10020
[10022]: https://github.com/bevyengine/bevy/pull/10022
[10023]: https://github.com/bevyengine/bevy/pull/10023
[10026]: https://github.com/bevyengine/bevy/pull/10026
[10030]: https://github.com/bevyengine/bevy/pull/10030
[10031]: https://github.com/bevyengine/bevy/pull/10031
[10035]: https://github.com/bevyengine/bevy/pull/10035
[10044]: https://github.com/bevyengine/bevy/pull/10044
[10051]: https://github.com/bevyengine/bevy/pull/10051
[10053]: https://github.com/bevyengine/bevy/pull/10053
[10056]: https://github.com/bevyengine/bevy/pull/10056
[10058]: https://github.com/bevyengine/bevy/pull/10058
[10065]: https://github.com/bevyengine/bevy/pull/10065
[10068]: https://github.com/bevyengine/bevy/pull/10068
[10070]: https://github.com/bevyengine/bevy/pull/10070
[10074]: https://github.com/bevyengine/bevy/pull/10074
[10078]: https://github.com/bevyengine/bevy/pull/10078
[10079]: https://github.com/bevyengine/bevy/pull/10079
[10080]: https://github.com/bevyengine/bevy/pull/10080
[10090]: https://github.com/bevyengine/bevy/pull/10090
[10092]: https://github.com/bevyengine/bevy/pull/10092
[10100]: https://github.com/bevyengine/bevy/pull/10100
[10103]: https://github.com/bevyengine/bevy/pull/10103
[10105]: https://github.com/bevyengine/bevy/pull/10105
[10109]: https://github.com/bevyengine/bevy/pull/10109
[10111]: https://github.com/bevyengine/bevy/pull/10111
[10117]: https://github.com/bevyengine/bevy/pull/10117
[10119]: https://github.com/bevyengine/bevy/pull/10119
[10121]: https://github.com/bevyengine/bevy/pull/10121
[10123]: https://github.com/bevyengine/bevy/pull/10123
[10124]: https://github.com/bevyengine/bevy/pull/10124
[10128]: https://github.com/bevyengine/bevy/pull/10128
[10132]: https://github.com/bevyengine/bevy/pull/10132
[10136]: https://github.com/bevyengine/bevy/pull/10136
[10142]: https://github.com/bevyengine/bevy/pull/10142
[10143]: https://github.com/bevyengine/bevy/pull/10143
[10144]: https://github.com/bevyengine/bevy/pull/10144
[10145]: https://github.com/bevyengine/bevy/pull/10145
[10148]: https://github.com/bevyengine/bevy/pull/10148
[10150]: https://github.com/bevyengine/bevy/pull/10150
[10151]: https://github.com/bevyengine/bevy/pull/10151
[10152]: https://github.com/bevyengine/bevy/pull/10152
[10156]: https://github.com/bevyengine/bevy/pull/10156
[10158]: https://github.com/bevyengine/bevy/pull/10158
[10160]: https://github.com/bevyengine/bevy/pull/10160
[10161]: https://github.com/bevyengine/bevy/pull/10161
[10163]: https://github.com/bevyengine/bevy/pull/10163
[10168]: https://github.com/bevyengine/bevy/pull/10168
[10171]: https://github.com/bevyengine/bevy/pull/10171
[10173]: https://github.com/bevyengine/bevy/pull/10173
[10174]: https://github.com/bevyengine/bevy/pull/10174
[10178]: https://github.com/bevyengine/bevy/pull/10178
[10179]: https://github.com/bevyengine/bevy/pull/10179
[10180]: https://github.com/bevyengine/bevy/pull/10180
[10182]: https://github.com/bevyengine/bevy/pull/10182
[10184]: https://github.com/bevyengine/bevy/pull/10184
[10185]: https://github.com/bevyengine/bevy/pull/10185
[10186]: https://github.com/bevyengine/bevy/pull/10186
[10189]: https://github.com/bevyengine/bevy/pull/10189
[10191]: https://github.com/bevyengine/bevy/pull/10191
[10194]: https://github.com/bevyengine/bevy/pull/10194
[10195]: https://github.com/bevyengine/bevy/pull/10195
[10196]: https://github.com/bevyengine/bevy/pull/10196
[10197]: https://github.com/bevyengine/bevy/pull/10197
[10198]: https://github.com/bevyengine/bevy/pull/10198
[10200]: https://github.com/bevyengine/bevy/pull/10200
[10202]: https://github.com/bevyengine/bevy/pull/10202
[10204]: https://github.com/bevyengine/bevy/pull/10204
[10206]: https://github.com/bevyengine/bevy/pull/10206
[10208]: https://github.com/bevyengine/bevy/pull/10208
[10210]: https://github.com/bevyengine/bevy/pull/10210
[10211]: https://github.com/bevyengine/bevy/pull/10211
[10217]: https://github.com/bevyengine/bevy/pull/10217
[10221]: https://github.com/bevyengine/bevy/pull/10221
[10222]: https://github.com/bevyengine/bevy/pull/10222
[10223]: https://github.com/bevyengine/bevy/pull/10223
[10226]: https://github.com/bevyengine/bevy/pull/10226
[10227]: https://github.com/bevyengine/bevy/pull/10227
[10230]: https://github.com/bevyengine/bevy/pull/10230
[10238]: https://github.com/bevyengine/bevy/pull/10238
[10240]: https://github.com/bevyengine/bevy/pull/10240
[10249]: https://github.com/bevyengine/bevy/pull/10249
[10251]: https://github.com/bevyengine/bevy/pull/10251
[10252]: https://github.com/bevyengine/bevy/pull/10252
[10253]: https://github.com/bevyengine/bevy/pull/10253
[10254]: https://github.com/bevyengine/bevy/pull/10254
[10259]: https://github.com/bevyengine/bevy/pull/10259
[10262]: https://github.com/bevyengine/bevy/pull/10262
[10264]: https://github.com/bevyengine/bevy/pull/10264
[10270]: https://github.com/bevyengine/bevy/pull/10270
[10271]: https://github.com/bevyengine/bevy/pull/10271
[10275]: https://github.com/bevyengine/bevy/pull/10275
[10276]: https://github.com/bevyengine/bevy/pull/10276
[10279]: https://github.com/bevyengine/bevy/pull/10279
[10286]: https://github.com/bevyengine/bevy/pull/10286
[10289]: https://github.com/bevyengine/bevy/pull/10289
[10294]: https://github.com/bevyengine/bevy/pull/10294
[10295]: https://github.com/bevyengine/bevy/pull/10295
[10297]: https://github.com/bevyengine/bevy/pull/10297
[10299]: https://github.com/bevyengine/bevy/pull/10299
[10303]: https://github.com/bevyengine/bevy/pull/10303
[10304]: https://github.com/bevyengine/bevy/pull/10304
[10306]: https://github.com/bevyengine/bevy/pull/10306
[10310]: https://github.com/bevyengine/bevy/pull/10310
[10311]: https://github.com/bevyengine/bevy/pull/10311
[10312]: https://github.com/bevyengine/bevy/pull/10312
[10326]: https://github.com/bevyengine/bevy/pull/10326
[10330]: https://github.com/bevyengine/bevy/pull/10330
[10339]: https://github.com/bevyengine/bevy/pull/10339
[10341]: https://github.com/bevyengine/bevy/pull/10341
[10345]: https://github.com/bevyengine/bevy/pull/10345
[10346]: https://github.com/bevyengine/bevy/pull/10346
[10352]: https://github.com/bevyengine/bevy/pull/10352
[10356]: https://github.com/bevyengine/bevy/pull/10356
[10358]: https://github.com/bevyengine/bevy/pull/10358
[10360]: https://github.com/bevyengine/bevy/pull/10360

## Version 0.11.0 (2023-07-09)

### Rendering

- [Webgpu support][8336]
- [improve shader import model][5703]
- [Screen Space Ambient Occlusion (SSAO) MVP][7402]
- [Temporal Antialiasing (TAA)][7291]
- [Immediate Mode Line/Gizmo Drawing][6529]
- [Make render graph slots optional for most cases][8109]
- [Split opaque and transparent phases][8090]
- [Built-in skybox][8275]
- [Add parallax mapping to bevy PBR][5928]
- [Add port of AMD's Robust Contrast Adaptive Sharpening][7422]
- [Add RenderGraphApp to simplify adding render nodes][8007]
- [Add screenshot api][7163]
- [Add morph targets][8158]
- [Screenshots in wasm][8455]
- [Add ViewNode to simplify render node management][8118]
- [Bias texture mipmaps][7614]
- [Instanced line rendering for gizmos based on `bevy_polyline`][8427]
- [Add `RenderTarget::TextureView`][8042]
- [Change default tonemapping method][8685]
- [Allow custom depth texture usage][6815]
- [Use the prepass normal texture in main pass when possible][8231]
- [Left-handed y-up cubemap coordinates][8122]
- [Allow SPIR-V shaders to process when shader defs are present][7772]
- [Remove unnecesssary values Vec from DynamicUniformBuffer and DynamicStorageBuffer][8299]
- [Add `MAY_DISCARD` shader def, enabling early depth tests for most cases][6697]
- [Add `Aabb` calculation for `Sprite`, `TextureAtlasSprite` and `Mesh2d`][7885]
- [Color::Lcha constructors][8041]
- [Fix Color::as_rgba_linear for Color::Lcha][8040]
- [Added Globals struct to prepass shader][8070]
- [Derive Copy and Clone for Collision][8121]
- [Fix crash when enabling HDR on 2d cameras][8151]
- [Dither fix][7977]
- [Compute `vertex_count` for indexed meshes on `GpuMesh`][8460]
- [Run update_previous_view_projections in PreUpdate schedule][9024]
- [Added `WebP` image format support][8220]
- [Add support for pnm textures][8601]
- [fix invalid bone weights][8316]
- [Fix pbr shader breaking on missing UVs][8412]
- [Fix Plane UVs / texture flip][8878]
- [Fix look_to resulting in NaN rotations][7817]
- [Fix look_to variable naming][8627]
- [Fix segfault with 2d gizmos][8223]
- [Use RenderGraphApp in more places][8298]
- [Fix viewport change detection][8323]
- [Remove capacity fields from all Buffer wrapper types][8301]
- [Sync pbr_types.wgsl StandardMaterial values][8380]
- [Avoid spawning gizmo meshes when no gizmos are being drawn][8180]
- [Use a consistent seed for AABB gizmo colors][9030]
- [bevy_pbr: Do not cull meshes without Aabbs from cascades][8444]
- [Handle vertex_uvs if they are present in default prepass fragment shader][8330]
- [Changed (Vec2, Vec2) to Rect in Camera::logical_viewport_rect][7867]
- [make glsl and spirv support optional][8491]
- [fix prepass normal_mapping][8978]
- [conversions between [u8; 4] and Color][8564]
- [Add option to disable gizmo rendering for specific cameras][8952]
- [Fix morph target prepass shader][9013]
- [Fix bloom wasm support][8631]
- [Fix black spots appearing due to NANs when SSAO is enabled][8926]
- [fix normal prepass][8890]
- [Refs #8975 -- Add return to RenderDevice::poll()][8977]
- [Fix WebGL mode for Adreno GPUs][8508]
- [Fix parallax mapping][9003]
- [Added Vec append to BufferVec - Issue #3531][8575]
- [Fix CAS shader with explicit FullscreenVertexOutput import][8993]
- [Make `TextureAtlas::texture_handles` `pub` instead of `pub(crate)` (#8633)][8643]
- [Make Material2d pipeline systems public][8642]
- [Fix screenshots on Wayland + Nvidia][8701]
- [Apply codebase changes in preparation for `StandardMaterial` transmission][8704]
- [Use ViewNode for TAA][8732]
- [Change Camera3dBundle::tonemapping to Default][8753]
- [Remove `Component` derive for AlphaMode][8804]
- [Make setup of Opaque3dPrepass and AlphaMask3dPrepass phase items consistent with others][8408]
- [Rename `Plane` struct to `HalfSpace`][8744]
- [Expand `FallbackImage` to include a `GpuImage` for each possible `TextureViewDimension`][6974]
- [Cascaded shadow maps: Fix prepass ortho depth clamping][8877]
- [Fix gizmos in WebGPU][8910]
- [Fix AsBindGroup derive, texture attribute, visibility flag parsing][8868]
- [Disable camera on window close][8802]
- [Reflect `Component` and `Default` of `BloomSettings`][8283]
- [Add Reflection Macros to TextureAtlasSprite][8428]
- [Implement Reflect on NoFrustumCulling][8801]

### Audio

- [ECS-based API redesign][8424]
- [Ability to set a Global Volume][7706]
- [Expose `AudioSink::empty()`][8145]

### Diagnostics

- [Allow systems using Diagnostics to run in parallel][8677]
- [add a feature for memory tracing with tracy][8272]
- [Re-add the "frame" span for tracy comparisons][8362]
- [log to stderr instead of stdout][8886]

### Scenes

- [bevy_scene: Add SceneFilter][6793]
- [(De) serialize resources in scenes][6846]
- [add position to scene errors][8065]
- [Bugfix: Scene reload fix (nonbreaking)][7951]
- [avoid panic with parented scenes on deleted entities][8512]

### Transform + Hierarchy

- [Fix transform propagation of orphaned entities][7264]

### Gizmo

- [Add a bounding box gizmo][8468]
- [Added `arc_2d` function for gizmos][8448]
- [Use AHash to get color from entity in bevy_gizmos][8960]
- [do not crash when rendering only one gizmo][8434]

### Reflection

- [reflect: stable type path v2][7184]
- [bevy_reflect: Better proxies][6971]
- [bevy_reflect: FromReflect Ergonomics Implementation][6056]
- [bevy_reflect: Allow `#[reflect(default)]` on enum variant fields][8514]
- [Add FromReflect where Reflect is used][8776]
- [Add get_at_mut to bevy_reflect::Map trait][8691]
- [Reflect now requires DynamicTypePath. Remove Reflect::get_type_path()][8764]
- [bevy_ui: Add `FromReflect` derives][8495]
- [Add Reflect and FromReflect for AssetPath][8531]
- [bevy_reflect: Fix trailing comma breaking derives][8014]
- [Fix Box dyn Reflect struct with a hashmap in it panicking when clone_value is called on it][8184]
- [bevy_reflect: Add `ReflectFromReflect` to the prelude][8496]
- [bevy_reflect: Allow construction of MapIter outside of the bevy_reflect crate.][8723]
- [bevy_reflect: Disambiguate type bounds in where clauses.][8761]
- [adding reflection for Cow<'static, [T]>][7454]
- [Do not require mut on ParsedPath::element_mut][8891]
- [Reflect UUID][8905]
- [Don't ignore additional entries in `UntypedReflectDeserializerVisitor`][7112]
- [Construct Box dyn Reflect from world for ReflectComponent][7407]
- [reflect: avoid deadlock in GenericTypeCell][8957]

### App

- [Allow tuples and single plugins in `add_plugins`, deprecate `add_plugin`][8097]
- [Merge ScheduleRunnerSettings into ScheduleRunnerPlugin][8585]
- [correctly setup everything in the default run_once runner][8740]
- [Fix `Plugin::build` detection][8103]
- [Fix not calling App::finish and App::cleanup in `ScheduleRunnerPlugin`][9054]
- [Relaxed runner type from Fn to FnOnce][8961]
- [Relax FnMut to FnOnce in app::edit_schedule][8982]

### Windowing + Reflection

- [Register missing types in bevy_window][7993]
- [bevy_reflect: implement Reflect for SmolStr][8771]

### Hierarchy

- [fix panic when moving child][8346]
- [Remove `Children` component when calling `despawn_descendants`][8476]
- [Change `despawn_descendants` to return `&mut Self`][8928]

### Time

- [Fix timer with zero duration][8467]

### Assets

- [Delay asset hot reloading][8503]
- [Add support for custom glTF vertex attributes.][5370]
- [Fix panic when using debug_asset_server][8485]
- [`unused_variables` warning when building with `filesystem_watcher` feature disabled][7938]
- [bevy_asset: Add `LoadContext::get_handle_untyped`][8470]

### Windowing

- [Move cursor position to internal state][7988]
- [Set cursor hittest during window creation][7966]
- [do not set hit test unconditionally on window creation][7996]
- [Add winit's `wayland-csd-adwaita` feature to Bevy's `wayland` feature][8722]
- [Support to set window theme and expose system window theme changed event][8593]
- [Touchpad magnify and rotate events][8791]
- [Fix windows not being centered properly when system interface is scaled][8903]
- [Expose WindowDestroyed events][9016]

### Animation

- [Register bevy_animation::PlayingAnimation][9023]

### UI

- [Ui Node Borders][7795]
- [Add CSS Grid support to `bevy_ui`][8026]
- [`text_system` split][7779]
- [Replace the local text queues in the text systems with flags stored in a component][8549]
- [`NoWrap` `Text` feature][8947]
- [add a default font][8445]
- [UI texture atlas support][8822]
- [Improved UI render batching][8793]
- [Consistent screen-space coordinates][8306]
- [`UiImage` helper functions][8199]
- [Perform text scaling calculations per text, not per glyph][7819]
- [Fix size of clipped text glyphs.][8197]
- [Apply scale factor to  `ImageMeasure` sizes][8545]
- [Fix WebGPU error in "ui_pipeline" by adding a flat interpolate attribute][8933]
- [Rename Interaction::Clicked -> Interaction::Pressed][9027]
- [Flatten UI `Style` properties that use `Size` + remove `Size`][8548]
- [Split UI `Overflow` by axis][8095]
- [Add methods for calculating the size and postion of UI nodes][7930]
- [Skip the UV calculations for untextured UI nodes][7809]
- [Fix text measurement algorithm][8425]
- [Divide by UiScale when converting UI coordinates from physical to logical][8720]
- [`MeasureFunc` improvements][8402]
- [Expose sorting methods in `Children`][8522]
- [Fix min and max size using size value][7948]
- [Fix the `Text2d` text anchor's incorrect horizontal alignment][8019]
- [Remove `Val::Undefined`][7485]
- [`Val` viewport unit variants][8137]
- [Remove the corresponding measure from Taffy when a `CalculatedSize` component is removed.][8294]
- [`UiRect` axes constructor][7656]
- [Fix the UV calculations for clipped and flipped ImageNodes][8195]
- [Fix text systems broken when resolving merge conflicts in #8026][8422]
- [Allow `bevy_ui` crate to compile without the `text` feature enabled][8437]
- [Fix the double leaf node updates in `flex_node_system`][8264]
- [also import the default handle when feature disabled][8456]
- [`measure_text_system` text query fix][8466]
- [Fix panic in example: text_wrap_debug.rs][8497]
- [UI layout tree debug print][8521]
- [Fix `Node::physical_rect` and add a `physical_size` method][8551]
- [Perform `relative_cursor_position` calculation vectorwise in `ui_focus_system`][8795]
- [Add `UiRect::px()` and `UiRect::percent()` utils][8866]
- [Add missing dependencies to `bevy_text` feature][8920]
- [Remove "bevy_text" feature attributes on imports used by non-text systems][8907]
- [Growing UI nodes Fix][8931]

### ECS

- [Schedule-First: the new and improved add_systems][8079]
- [Add OnTransition schedule that is ran between OnExit and OnEnter][7936]
- [`run_if` for `SystemConfigs` via anonymous system sets][7676]
- [Remove OnUpdate system set][8260]
- [Rename apply_system_buffers to apply_deferred][8726]
- [Rename Command's "write" method to "apply"][8814]
- [Require `#[derive(Event)]` on all Events][7086]
- [Implement WorldQuery for EntityRef][6960]
- [Improve or-with disjoint checks][7085]
- [Add a method to run read-only systems using `&World`][8849]
- [Reduce branching when inserting components][8053]
- [Make `#[system_param(ignore)]` and `#[world_query(ignore)]` unnecessary][8030]
- [Remove `#[system_param(ignore)]` and `#[world_query(ignore)]`][8265]
- [Extend the `WorldQuery` macro to tuple structs][8119]
- [Make state private and only accessible through getter for State resource][8009]
- [implement `Deref` for `State<S>`][8668]
- [Inline more ECS functions][8083]
- [Add a `scope` API for world schedules][8387]
- [Simplify system piping and make it more flexible][8377]
- [Add `any_component_removed` condition][8326]
- [Use `UnsafeWorldCell` to increase code quality for `SystemParam`][8174]
- [Improve safety for the multi-threaded executor using `UnsafeWorldCell`][8292]
- [Migrate the rest of the engine to `UnsafeWorldCell`][8833]
- [Make the `Condition` trait generic][8721]
- [Add or_else combinator to run_conditions.rs][8714]
- [Add iter_many_manual QueryState method][8772]
- [Provide access to world storages via UnsafeWorldCell][8987]
- [Added Has T WorldQuery type][8844]
- [Add/fix `track_caller` attribute on panicking entity accessor methods][8951]
- [Increase type safety and clarity for change detection][7905]
- [Make `WorldQuery` meta types unnameable][7964]
- [Add a public constructor for `Mut<T>`][7931]
- [Remove ChangeTrackers][7902]
- [Derive Eq, PartialEq for Tick][9020]
- [Initialize empty schedules when calling `.in_schedule` if they do not already exist][7911]
- [Replace multiple calls to `add_system` with `add_systems`][8001]
- [don't panic on unknown ambiguity][7950]
- [add Clone to common conditions][8060]
- [Make BundleInfo's fields not pub(crate)][8068]
- [Pass query change ticks to `QueryParIter` instead of always using change ticks from `World`.][8029]
- [Remove redundant bounds check in `Entities::get`][8108]
- [Add World::try_run_schedule][8028]
- [change not implemation to custom system struct][8105]
- [Fix name conflicts caused by the `SystemParam` and `WorldQuery` macros][8012]
- [Check for conflicting accesses in `assert_is_system`][8154]
- [Fix field visibility for read-only `WorldQuery` types][8163]
- [`Or<T>` should be a new type of `PhantomData<T>`][8212]
- [Make standard commands more ergonomic (in niche cases)][8249]
- [Remove base set error variants of `ScheduleBuildError`][8269]
- [Replace some unsafe system executor code with safe code][8274]
- [Update `increment_change_tick` to return a strongly-typed `Tick`][8295]
- [Move event traces to detailed_trace!][7732]
- [Only trigger state transitons if `next_state != old_state`][8359]
- [Fix panics and docs when using World schedules][8364]
- [Improve warning for Send resources marked as non_send][8000]
- [Reorganize system modules][8419]
- [Fix boxed labels][8436]
- [Simplify world schedule methods][8403]
- [Just print out name string, not the entire Name struct][8494]
- [Manually implement common traits for `EventId`][8529]
- [Replace remaining uses of `&T, Changed<T>` with `Ref` in UI system queries][8567]
- [Rename `UnsafeWorldCell::read_change_tick`][8588]
- [Improve encapsulation for commands and add docs][8725]
- [Fix all_tuples + added docs.][8743]
- [Add `new` and `map` methods to `Ref`][8797]
- [Allow unsized types as mapped value in `Ref::map`][8817]
- [Implement `Clone` for `CombinatorSystem`][8826]
- [Add get_ref to EntityRef][8818]
- [Make `QueryParIter::for_each_unchecked` private][8848]
- [Simplify the `ComponentIdFor` type][8845]
- [Add last_changed_tick and added_tick to ComponentTicks][8803]
- [Require read-only queries in `QueryState::par_iter`][8832]
- [Fix any_component_removed][8939]
- [Deprecate type aliases for `WorldQuery::Fetch`][8843]
- [bevy_ecs: add untyped methods for inserting components and bundles][7204]
- [Move AppTypeRegistry to bevy_ecs][8901]
- [skip check change tick for apply_deferred systems][8760]
- [Split the bevy_ecs reflect.rs module][8834]
- [Make function pointers of ecs Reflect* public][8687]

### Rendering + Reflection + Scenes

- [fix: register Cascade in the TypeRegistry][8088]

### Tasks

- [Add optional single-threaded feature to bevy_ecs/bevy_tasks][6690]

### Math

- [Re-export glam_assert feature][8232]
- [Fix CubicCurve::iter_samples iteration count][8049]
- [Add integer equivalents for `Rect`][7984]
- [Add `CubicCurve::segment_count` + `iter_samples` adjustment][8711]

### Rendering + Assets + Meta

- [Add depending bevy features for higher level one][7855]

### ECS + Scenes

- [Make scene handling of entity references robust][7335]
- [Rename map_entities and map_specific_entities][7570]

### Util

- [bevy_derive: Add `#[deref]` attribute][8552]

### Input

- [Add gamepad rumble support to bevy_input][8398]
- [Rename keys like `LAlt` to `AltLeft`][8792]
- [Add window entity to mouse and keyboard events][8852]
- [Add get_unclamped to Axis][8871]

### Upgrades

- [Upgrade Taffy requirement to v0.3.5][7959]
- [Update ruzstd and basis universal][8622]
- [Updated to wgpu 0.16.0, wgpu-hal 0.16.0 and naga 0.12.0][8446]
- [Update sysinfo requirement from 0.28.1 to 0.29.0][8650]
- [Update libloading requirement from 0.7 to 0.8][8649]
- [update syn, encase, glam and hexasphere][8573]
- [Update android_log-sys requirement from 0.2.0 to 0.3.0][7925]
- [update bitflags to 2.3][8728]
- [Update ruzstd requirement from 0.3.1 to 0.4.0][8755]
- [Update notify requirement from 5.0.0 to 6.0.0][8757]
- [Bump hashbrown to 0.14][8904]
- [update ahash and hashbrown][8623]
- [Bump accesskit and accesskit_winit][8655]

### Examples

- [new example showcase tool][8561]
- [Adding a bezier curve example][8194]
- [Add low level post process example using a custom render pass][6909]
- [Add example to demonstrate manual generation and UV mapping of 3D mesh (generate_custom_mesh) solve #4922][8909]
- [Add `overflow_debug` example][8198]
- [UI text wrapping and `LineBreakOn` example][7761]
- [Size Constraints Example][7956]
- [UI Display and Visibility Example][7629]

[5370]: https://github.com/bevyengine/bevy/pull/5370
[5703]: https://github.com/bevyengine/bevy/pull/5703
[5928]: https://github.com/bevyengine/bevy/pull/5928
[6529]: https://github.com/bevyengine/bevy/pull/6529
[6697]: https://github.com/bevyengine/bevy/pull/6697
[6815]: https://github.com/bevyengine/bevy/pull/6815
[6846]: https://github.com/bevyengine/bevy/pull/6846
[6909]: https://github.com/bevyengine/bevy/pull/6909
[6960]: https://github.com/bevyengine/bevy/pull/6960
[6971]: https://github.com/bevyengine/bevy/pull/6971
[6974]: https://github.com/bevyengine/bevy/pull/6974
[7085]: https://github.com/bevyengine/bevy/pull/7085
[7086]: https://github.com/bevyengine/bevy/pull/7086
[7112]: https://github.com/bevyengine/bevy/pull/7112
[7163]: https://github.com/bevyengine/bevy/pull/7163
[7184]: https://github.com/bevyengine/bevy/pull/7184
[7204]: https://github.com/bevyengine/bevy/pull/7204
[7264]: https://github.com/bevyengine/bevy/pull/7264
[7291]: https://github.com/bevyengine/bevy/pull/7291
[7335]: https://github.com/bevyengine/bevy/pull/7335
[7402]: https://github.com/bevyengine/bevy/pull/7402
[7407]: https://github.com/bevyengine/bevy/pull/7407
[7422]: https://github.com/bevyengine/bevy/pull/7422
[7454]: https://github.com/bevyengine/bevy/pull/7454
[7485]: https://github.com/bevyengine/bevy/pull/7485
[7570]: https://github.com/bevyengine/bevy/pull/7570
[7614]: https://github.com/bevyengine/bevy/pull/7614
[7629]: https://github.com/bevyengine/bevy/pull/7629
[7656]: https://github.com/bevyengine/bevy/pull/7656
[7676]: https://github.com/bevyengine/bevy/pull/7676
[7706]: https://github.com/bevyengine/bevy/pull/7706
[7732]: https://github.com/bevyengine/bevy/pull/7732
[7761]: https://github.com/bevyengine/bevy/pull/7761
[7772]: https://github.com/bevyengine/bevy/pull/7772
[7779]: https://github.com/bevyengine/bevy/pull/7779
[7795]: https://github.com/bevyengine/bevy/pull/7795
[7809]: https://github.com/bevyengine/bevy/pull/7809
[7817]: https://github.com/bevyengine/bevy/pull/7817
[7819]: https://github.com/bevyengine/bevy/pull/7819
[7855]: https://github.com/bevyengine/bevy/pull/7855
[7867]: https://github.com/bevyengine/bevy/pull/7867
[7885]: https://github.com/bevyengine/bevy/pull/7885
[7902]: https://github.com/bevyengine/bevy/pull/7902
[7905]: https://github.com/bevyengine/bevy/pull/7905
[7911]: https://github.com/bevyengine/bevy/pull/7911
[7925]: https://github.com/bevyengine/bevy/pull/7925
[7930]: https://github.com/bevyengine/bevy/pull/7930
[7931]: https://github.com/bevyengine/bevy/pull/7931
[7936]: https://github.com/bevyengine/bevy/pull/7936
[7938]: https://github.com/bevyengine/bevy/pull/7938
[7948]: https://github.com/bevyengine/bevy/pull/7948
[7950]: https://github.com/bevyengine/bevy/pull/7950
[7951]: https://github.com/bevyengine/bevy/pull/7951
[7956]: https://github.com/bevyengine/bevy/pull/7956
[7959]: https://github.com/bevyengine/bevy/pull/7959
[7964]: https://github.com/bevyengine/bevy/pull/7964
[7966]: https://github.com/bevyengine/bevy/pull/7966
[7977]: https://github.com/bevyengine/bevy/pull/7977
[7984]: https://github.com/bevyengine/bevy/pull/7984
[7988]: https://github.com/bevyengine/bevy/pull/7988
[7993]: https://github.com/bevyengine/bevy/pull/7993
[7996]: https://github.com/bevyengine/bevy/pull/7996
[8000]: https://github.com/bevyengine/bevy/pull/8000
[8001]: https://github.com/bevyengine/bevy/pull/8001
[8007]: https://github.com/bevyengine/bevy/pull/8007
[8009]: https://github.com/bevyengine/bevy/pull/8009
[8012]: https://github.com/bevyengine/bevy/pull/8012
[8014]: https://github.com/bevyengine/bevy/pull/8014
[8019]: https://github.com/bevyengine/bevy/pull/8019
[8026]: https://github.com/bevyengine/bevy/pull/8026
[8028]: https://github.com/bevyengine/bevy/pull/8028
[8029]: https://github.com/bevyengine/bevy/pull/8029
[8030]: https://github.com/bevyengine/bevy/pull/8030
[8040]: https://github.com/bevyengine/bevy/pull/8040
[8041]: https://github.com/bevyengine/bevy/pull/8041
[8042]: https://github.com/bevyengine/bevy/pull/8042
[8049]: https://github.com/bevyengine/bevy/pull/8049
[8053]: https://github.com/bevyengine/bevy/pull/8053
[8060]: https://github.com/bevyengine/bevy/pull/8060
[8065]: https://github.com/bevyengine/bevy/pull/8065
[8068]: https://github.com/bevyengine/bevy/pull/8068
[8070]: https://github.com/bevyengine/bevy/pull/8070
[8079]: https://github.com/bevyengine/bevy/pull/8079
[8083]: https://github.com/bevyengine/bevy/pull/8083
[8088]: https://github.com/bevyengine/bevy/pull/8088
[8090]: https://github.com/bevyengine/bevy/pull/8090
[8095]: https://github.com/bevyengine/bevy/pull/8095
[8097]: https://github.com/bevyengine/bevy/pull/8097
[8103]: https://github.com/bevyengine/bevy/pull/8103
[8105]: https://github.com/bevyengine/bevy/pull/8105
[8108]: https://github.com/bevyengine/bevy/pull/8108
[8109]: https://github.com/bevyengine/bevy/pull/8109
[8118]: https://github.com/bevyengine/bevy/pull/8118
[8119]: https://github.com/bevyengine/bevy/pull/8119
[8121]: https://github.com/bevyengine/bevy/pull/8121
[8122]: https://github.com/bevyengine/bevy/pull/8122
[8137]: https://github.com/bevyengine/bevy/pull/8137
[8145]: https://github.com/bevyengine/bevy/pull/8145
[8151]: https://github.com/bevyengine/bevy/pull/8151
[8154]: https://github.com/bevyengine/bevy/pull/8154
[8158]: https://github.com/bevyengine/bevy/pull/8158
[8163]: https://github.com/bevyengine/bevy/pull/8163
[8174]: https://github.com/bevyengine/bevy/pull/8174
[8180]: https://github.com/bevyengine/bevy/pull/8180
[8184]: https://github.com/bevyengine/bevy/pull/8184
[8194]: https://github.com/bevyengine/bevy/pull/8194
[8195]: https://github.com/bevyengine/bevy/pull/8195
[8197]: https://github.com/bevyengine/bevy/pull/8197
[8198]: https://github.com/bevyengine/bevy/pull/8198
[8199]: https://github.com/bevyengine/bevy/pull/8199
[8212]: https://github.com/bevyengine/bevy/pull/8212
[8220]: https://github.com/bevyengine/bevy/pull/8220
[8223]: https://github.com/bevyengine/bevy/pull/8223
[8231]: https://github.com/bevyengine/bevy/pull/8231
[8232]: https://github.com/bevyengine/bevy/pull/8232
[8249]: https://github.com/bevyengine/bevy/pull/8249
[8260]: https://github.com/bevyengine/bevy/pull/8260
[8264]: https://github.com/bevyengine/bevy/pull/8264
[8265]: https://github.com/bevyengine/bevy/pull/8265
[8269]: https://github.com/bevyengine/bevy/pull/8269
[8272]: https://github.com/bevyengine/bevy/pull/8272
[8274]: https://github.com/bevyengine/bevy/pull/8274
[8275]: https://github.com/bevyengine/bevy/pull/8275
[8283]: https://github.com/bevyengine/bevy/pull/8283
[8292]: https://github.com/bevyengine/bevy/pull/8292
[8294]: https://github.com/bevyengine/bevy/pull/8294
[8295]: https://github.com/bevyengine/bevy/pull/8295
[8298]: https://github.com/bevyengine/bevy/pull/8298
[8299]: https://github.com/bevyengine/bevy/pull/8299
[8301]: https://github.com/bevyengine/bevy/pull/8301
[8306]: https://github.com/bevyengine/bevy/pull/8306
[8316]: https://github.com/bevyengine/bevy/pull/8316
[8323]: https://github.com/bevyengine/bevy/pull/8323
[8326]: https://github.com/bevyengine/bevy/pull/8326
[8330]: https://github.com/bevyengine/bevy/pull/8330
[8336]: https://github.com/bevyengine/bevy/pull/8336
[8346]: https://github.com/bevyengine/bevy/pull/8346
[8359]: https://github.com/bevyengine/bevy/pull/8359
[8362]: https://github.com/bevyengine/bevy/pull/8362
[8364]: https://github.com/bevyengine/bevy/pull/8364
[8377]: https://github.com/bevyengine/bevy/pull/8377
[8380]: https://github.com/bevyengine/bevy/pull/8380
[8387]: https://github.com/bevyengine/bevy/pull/8387
[8398]: https://github.com/bevyengine/bevy/pull/8398
[8402]: https://github.com/bevyengine/bevy/pull/8402
[8403]: https://github.com/bevyengine/bevy/pull/8403
[8408]: https://github.com/bevyengine/bevy/pull/8408
[8412]: https://github.com/bevyengine/bevy/pull/8412
[8419]: https://github.com/bevyengine/bevy/pull/8419
[8422]: https://github.com/bevyengine/bevy/pull/8422
[8425]: https://github.com/bevyengine/bevy/pull/8425
[8427]: https://github.com/bevyengine/bevy/pull/8427
[8428]: https://github.com/bevyengine/bevy/pull/8428
[8434]: https://github.com/bevyengine/bevy/pull/8434
[8436]: https://github.com/bevyengine/bevy/pull/8436
[8437]: https://github.com/bevyengine/bevy/pull/8437
[8444]: https://github.com/bevyengine/bevy/pull/8444
[8445]: https://github.com/bevyengine/bevy/pull/8445
[8446]: https://github.com/bevyengine/bevy/pull/8446
[8448]: https://github.com/bevyengine/bevy/pull/8448
[8455]: https://github.com/bevyengine/bevy/pull/8455
[8456]: https://github.com/bevyengine/bevy/pull/8456
[8460]: https://github.com/bevyengine/bevy/pull/8460
[8466]: https://github.com/bevyengine/bevy/pull/8466
[8467]: https://github.com/bevyengine/bevy/pull/8467
[8468]: https://github.com/bevyengine/bevy/pull/8468
[8470]: https://github.com/bevyengine/bevy/pull/8470
[8476]: https://github.com/bevyengine/bevy/pull/8476
[8485]: https://github.com/bevyengine/bevy/pull/8485
[8491]: https://github.com/bevyengine/bevy/pull/8491
[8494]: https://github.com/bevyengine/bevy/pull/8494
[8495]: https://github.com/bevyengine/bevy/pull/8495
[8496]: https://github.com/bevyengine/bevy/pull/8496
[8497]: https://github.com/bevyengine/bevy/pull/8497
[8503]: https://github.com/bevyengine/bevy/pull/8503
[8512]: https://github.com/bevyengine/bevy/pull/8512
[8514]: https://github.com/bevyengine/bevy/pull/8514
[8521]: https://github.com/bevyengine/bevy/pull/8521
[8522]: https://github.com/bevyengine/bevy/pull/8522
[8529]: https://github.com/bevyengine/bevy/pull/8529
[8531]: https://github.com/bevyengine/bevy/pull/8531
[8545]: https://github.com/bevyengine/bevy/pull/8545
[8548]: https://github.com/bevyengine/bevy/pull/8548
[8549]: https://github.com/bevyengine/bevy/pull/8549
[8551]: https://github.com/bevyengine/bevy/pull/8551
[8552]: https://github.com/bevyengine/bevy/pull/8552
[8561]: https://github.com/bevyengine/bevy/pull/8561
[8564]: https://github.com/bevyengine/bevy/pull/8564
[8567]: https://github.com/bevyengine/bevy/pull/8567
[8573]: https://github.com/bevyengine/bevy/pull/8573
[8575]: https://github.com/bevyengine/bevy/pull/8575
[8585]: https://github.com/bevyengine/bevy/pull/8585
[8588]: https://github.com/bevyengine/bevy/pull/8588
[8593]: https://github.com/bevyengine/bevy/pull/8593
[8601]: https://github.com/bevyengine/bevy/pull/8601
[8622]: https://github.com/bevyengine/bevy/pull/8622
[8623]: https://github.com/bevyengine/bevy/pull/8623
[8627]: https://github.com/bevyengine/bevy/pull/8627
[8631]: https://github.com/bevyengine/bevy/pull/8631
[8642]: https://github.com/bevyengine/bevy/pull/8642
[8643]: https://github.com/bevyengine/bevy/pull/8643
[8649]: https://github.com/bevyengine/bevy/pull/8649
[8650]: https://github.com/bevyengine/bevy/pull/8650
[8668]: https://github.com/bevyengine/bevy/pull/8668
[8677]: https://github.com/bevyengine/bevy/pull/8677
[8685]: https://github.com/bevyengine/bevy/pull/8685
[8687]: https://github.com/bevyengine/bevy/pull/8687
[8691]: https://github.com/bevyengine/bevy/pull/8691
[8701]: https://github.com/bevyengine/bevy/pull/8701
[8704]: https://github.com/bevyengine/bevy/pull/8704
[8711]: https://github.com/bevyengine/bevy/pull/8711
[8714]: https://github.com/bevyengine/bevy/pull/8714
[8721]: https://github.com/bevyengine/bevy/pull/8721
[8722]: https://github.com/bevyengine/bevy/pull/8722
[8723]: https://github.com/bevyengine/bevy/pull/8723
[8725]: https://github.com/bevyengine/bevy/pull/8725
[8726]: https://github.com/bevyengine/bevy/pull/8726
[8728]: https://github.com/bevyengine/bevy/pull/8728
[8732]: https://github.com/bevyengine/bevy/pull/8732
[8740]: https://github.com/bevyengine/bevy/pull/8740
[8743]: https://github.com/bevyengine/bevy/pull/8743
[8744]: https://github.com/bevyengine/bevy/pull/8744
[8753]: https://github.com/bevyengine/bevy/pull/8753
[8755]: https://github.com/bevyengine/bevy/pull/8755
[8757]: https://github.com/bevyengine/bevy/pull/8757
[8760]: https://github.com/bevyengine/bevy/pull/8760
[8761]: https://github.com/bevyengine/bevy/pull/8761
[8764]: https://github.com/bevyengine/bevy/pull/8764
[8771]: https://github.com/bevyengine/bevy/pull/8771
[8772]: https://github.com/bevyengine/bevy/pull/8772
[8776]: https://github.com/bevyengine/bevy/pull/8776
[8791]: https://github.com/bevyengine/bevy/pull/8791
[8792]: https://github.com/bevyengine/bevy/pull/8792
[8793]: https://github.com/bevyengine/bevy/pull/8793
[8795]: https://github.com/bevyengine/bevy/pull/8795
[8797]: https://github.com/bevyengine/bevy/pull/8797
[8801]: https://github.com/bevyengine/bevy/pull/8801
[8802]: https://github.com/bevyengine/bevy/pull/8802
[8803]: https://github.com/bevyengine/bevy/pull/8803
[8804]: https://github.com/bevyengine/bevy/pull/8804
[8814]: https://github.com/bevyengine/bevy/pull/8814
[8817]: https://github.com/bevyengine/bevy/pull/8817
[8818]: https://github.com/bevyengine/bevy/pull/8818
[8822]: https://github.com/bevyengine/bevy/pull/8822
[8826]: https://github.com/bevyengine/bevy/pull/8826
[8832]: https://github.com/bevyengine/bevy/pull/8832
[8833]: https://github.com/bevyengine/bevy/pull/8833
[8834]: https://github.com/bevyengine/bevy/pull/8834
[8843]: https://github.com/bevyengine/bevy/pull/8843
[8844]: https://github.com/bevyengine/bevy/pull/8844
[8845]: https://github.com/bevyengine/bevy/pull/8845
[8848]: https://github.com/bevyengine/bevy/pull/8848
[8849]: https://github.com/bevyengine/bevy/pull/8849
[8852]: https://github.com/bevyengine/bevy/pull/8852
[8866]: https://github.com/bevyengine/bevy/pull/8866
[8868]: https://github.com/bevyengine/bevy/pull/8868
[8871]: https://github.com/bevyengine/bevy/pull/8871
[8877]: https://github.com/bevyengine/bevy/pull/8877
[8878]: https://github.com/bevyengine/bevy/pull/8878
[8886]: https://github.com/bevyengine/bevy/pull/8886
[8890]: https://github.com/bevyengine/bevy/pull/8890
[8891]: https://github.com/bevyengine/bevy/pull/8891
[8901]: https://github.com/bevyengine/bevy/pull/8901
[8903]: https://github.com/bevyengine/bevy/pull/8903
[8904]: https://github.com/bevyengine/bevy/pull/8904
[8905]: https://github.com/bevyengine/bevy/pull/8905
[8907]: https://github.com/bevyengine/bevy/pull/8907
[8909]: https://github.com/bevyengine/bevy/pull/8909
[8910]: https://github.com/bevyengine/bevy/pull/8910
[8920]: https://github.com/bevyengine/bevy/pull/8920
[8928]: https://github.com/bevyengine/bevy/pull/8928
[8933]: https://github.com/bevyengine/bevy/pull/8933
[8939]: https://github.com/bevyengine/bevy/pull/8939
[8947]: https://github.com/bevyengine/bevy/pull/8947
[8951]: https://github.com/bevyengine/bevy/pull/8951
[8960]: https://github.com/bevyengine/bevy/pull/8960
[8957]: https://github.com/bevyengine/bevy/pull/8957
[9054]: https://github.com/bevyengine/bevy/pull/9054
[6690]: https://github.com/bevyengine/bevy/pull/6690
[8424]: https://github.com/bevyengine/bevy/pull/8424
[8655]: https://github.com/bevyengine/bevy/pull/8655
[6793]: https://github.com/bevyengine/bevy/pull/6793
[8720]: https://github.com/bevyengine/bevy/pull/8720
[9024]: https://github.com/bevyengine/bevy/pull/9024
[9027]: https://github.com/bevyengine/bevy/pull/9027
[9016]: https://github.com/bevyengine/bevy/pull/9016
[9023]: https://github.com/bevyengine/bevy/pull/9023
[9020]: https://github.com/bevyengine/bevy/pull/9020
[9030]: https://github.com/bevyengine/bevy/pull/9030
[9013]: https://github.com/bevyengine/bevy/pull/9013
[8926]: https://github.com/bevyengine/bevy/pull/8926
[9003]: https://github.com/bevyengine/bevy/pull/9003
[8993]: https://github.com/bevyengine/bevy/pull/8993
[8508]: https://github.com/bevyengine/bevy/pull/8508
[6056]: https://github.com/bevyengine/bevy/pull/6056
[8987]: https://github.com/bevyengine/bevy/pull/8987
[8952]: https://github.com/bevyengine/bevy/pull/8952
[8961]: https://github.com/bevyengine/bevy/pull/8961
[8978]: https://github.com/bevyengine/bevy/pull/8978
[8982]: https://github.com/bevyengine/bevy/pull/8982
[8977]: https://github.com/bevyengine/bevy/pull/8977
[8931]: https://github.com/bevyengine/bevy/pull/8931

## Version 0.10.0 (2023-03-06)

## Added

- [Accessibility: Added `Label` for marking text specifically as a label for UI controls.][6874]
- [Accessibility: Integrate with and expose AccessKit accessibility.][6874]
- [App: `App::setup`][7586]
- [App: `SubApp::new`][7290]
- [App: Bevy apps will now log system information on startup by default][5454]
- [Audio Expose symphonia features from rodio in bevy_audio and bevy][6388]
- [Audio: Basic spatial audio][6028]
- [ECS: `bevy_ptr::dangling_with_align`: creates a well-aligned dangling pointer to a type whose alignment is not known at compile time.][6618]
- [ECS: `Column::get_added_ticks`][6547]
- [ECS: `Column::get_column_ticks`][6547]
- [ECS: `DetectChanges::set_if_neq`: triggering change detection when the new and previous values are equal. This will work on both components and resources.][6853]
- [ECS: `SparseSet::get_added_ticks`][6547]
- [ECS: `SparseSet::get_column_ticks`][6547]
- [ECS: `Tick`, a wrapper around a single change detection tick.][6547]
- [ECS: `UnsafeWorldCell::world_mut` now exists and can be used to get a `&mut World` out of `UnsafeWorldCell`][7381]
- [ECS: `WorldId` now implements the `FromWorld` trait.][7726]
- [ECS: A `core::fmt::Pointer` impl to `Ptr`, `PtrMut` and `OwnedPtr`.][6980]
- [ECS: Add `bevy_ecs::schedule_v3` module][6587]
- [ECS: Add `EntityMap::iter()`][6935]
- [ECS: Add `Ref` to the prelude][7392]
- [ECS: Add `report_sets` option to `ScheduleBuildSettings`][7756]
- [ECS: add `Resources::iter` to iterate over all resource IDs][6592]
- [ECS: add `UnsafeWorldCell` abstraction][6404]
- [ECS: Add `World::clear_resources` & `World::clear_all`][3212]
- [ECS: Add a basic example for system ordering][7017]
- [ECS: Add a missing impl of `ReadOnlySystemParam` for `Option<NonSend<>>`][7245]
- [ECS: add a spawn_on_external method to allow spawning on the scope’s thread or an external thread][7415]
- [ECS: Add const `Entity::PLACEHOLDER`][6761]
- [ECS: Add example to show how to use `apply_system_buffers`][7793]
- [ECS: Add logging variants of system piping][6751]
- [ECS: Add safe constructors for untyped pointers `Ptr` and `PtrMut`][6539]
- [ECS: Add unit test with system that panics][7491]
- [ECS: Add wrapping_add to change_tick][7146]
- [ECS: Added “base sets” and ported CoreSet to use them.][7466]
- [ECS: Added `as_mut` and `as_ref` methods to `MutUntyped`.][7009]
- [ECS: Added `bevy::ecs::system::assert_is_read_only_system`.][7547]
- [ECS: Added `Components::resource_id`.][7284]
- [ECS: Added `DebugName` world query for more human friendly debug names of entities.][7186]
- [ECS: Added `distributive_run_if` to `IntoSystemConfigs` to enable adding a run condition to each system when using `add_systems`.][7724]
- [ECS: Added `EntityLocation::table_id`][6681]
- [ECS: Added `EntityLocation::table_row`.][6681]
- [ECS: Added `IntoIterator` implementation for `EventReader` so you can now do `&mut reader` instead of `reader.iter()` for events.][7720]
- [ECS: Added `len`, `is_empty`, `iter` methods on SparseSets.][7638]
- [ECS: Added `ManualEventReader::clear()`][7471]
- [ECS: Added `MutUntyped::with_type` which allows converting into a `Mut<T>`][7113]
- [ECS: Added `new_for_test` on `ComponentInfo` to make test code easy.][7638]
- [ECS: Added `not` condition.][7559]
- [ECS: Added `on_timer` and `on_fixed_timer` run conditions][7866]
- [ECS: Added `OwningPtr::read_unaligned`.][7039]
- [ECS: Added `ReadOnlySystem`, which is implemented for any `System` type whose parameters all implement `ReadOnlySystemParam`.][7547]
- [ECS: Added `Ref` which allows inspecting change detection flags in an immutable way][7097]
- [ECS: Added `shrink` and `as_ref` methods to `PtrMut`.][7009]
- [ECS: Added `SystemMeta::name`][6900]
- [ECS: Added `SystemState::get_manual_mut`][7084]
- [ECS: Added `SystemState::get_manual`][7084]
- [ECS: Added `SystemState::update_archetypes`][7084]
- [ECS: Added a large number of methods on `App` to work with schedules ergonomically][7267]
- [ECS: Added conversions from `Ptr`, `PtrMut`, and `OwningPtr` to `NonNull<u8>`.][7181]
- [ECS: Added rore common run conditions: `on_event`, resource change detection, `state_changed`, `any_with_component`][7579]
- [ECS: Added support for variants of `bevy_ptr` types that do not require being correctly aligned for the pointee type.][7151]
- [ECS: Added the `CoreSchedule` enum][7267]
- [ECS: Added the `SystemParam` type `Deferred<T>`, which can be used to defer `World` mutations. Powered by the new trait `SystemBuffer`.][6817]
- [ECS: Added the extension methods `.and_then(...)` and `.or_else(...)` to run conditions, which allows combining run conditions with short-circuiting behavior.][7605]
- [ECS: Added the marker trait `BaseSystemSet`, which is distinguished from a `FreeSystemSet`. These are both subtraits of `SystemSet`.][7863]
- [ECS: Added the method `reborrow` to `Mut`, `ResMut`, `NonSendMut`, and `MutUntyped`.][7114]
- [ECS: Added the private `prepare_view_uniforms` system now has a public system set for scheduling purposes, called `ViewSet::PrepareUniforms`][7267]
- [ECS: Added the trait `Combine`, which can be used with the new `CombinatorSystem` to create system combinators with custom behavior.][7605]
- [ECS: Added the trait `EntityCommand`. This is a counterpart of `Command` for types that execute code for a single entity.][7015]
- [ECS: introduce EntityLocation::INVALID const and adjust Entities::get comment][7623]
- [ECS: States derive macro][7535]
- [ECS: support for tuple structs and unit structs to the `SystemParam` derive macro.][6957]
- [Hierarchy: Add `Transform::look_to`][6692]
- [Hierarchy: Added `add_child`, `set_parent` and `remove_parent` to `EntityMut`][6926]
- [Hierarchy: Added `clear_children(&mut self) -> &mut Self` and `replace_children(&mut self, children: &[Entity]) -> &mut Self` function in `BuildChildren` trait][6035]
- [Hierarchy: Added `ClearChildren` and `ReplaceChildren` struct][6035]
- [Hierarchy: Added `push_and_replace_children_commands` and `push_and_clear_children_commands` test][6035]
- [Hierarchy: Added the `BuildChildrenTransformExt` trait][7024]
- [Input: add Input Method Editor support][7325]
- [Input: Added `Axis<T>::devices`][5400]
- [INput: Added common run conditions for `bevy_input`][7806]
- [Macro: add helper for macro to get either bevy::x or bevy_x depending on how it was imported][7164]
- [Math: `CubicBezier2d`, `CubicBezier3d`, `QuadraticBezier2d`, and `QuadraticBezier3d` types with methods for sampling position, velocity, and acceleration. The generic `Bezier` type is also available, and generic over any degree of Bezier curve.][7653]
- [Math: `CubicBezierEasing`, with additional methods to allow for smooth easing animations.][7653]
- [Math: Added a generic cubic curve trait, and implementation for Cardinal splines (including Catmull-Rom), B-Splines, Beziers, and Hermite Splines. 2D cubic curve segments also implement easing functionality for animation.][7683]
- [New reflection path syntax: struct field access by index (example syntax: `foo#1`)][7321]
- [Reflect  `State` generics other than just `RandomState` can now be reflected for both `hashbrown::HashMap` and `collections::HashMap`][7782]
- [Reflect: `Aabb` now implements `FromReflect`.][7396]
- [Reflect: `derive(Reflect)` now supports structs and enums that contain generic types][7364]
- [Reflect: `ParsedPath` for cached reflection paths][7321]
- [Reflect: `std::collections::HashMap` can now be reflected][7782]
- [Reflect: `std::collections::VecDeque` now implements `Reflect` and all relevant traits.][6831]
- [Reflect: Add reflection path support for `Tuple` types][7324]
- [Reflect: Added `ArrayIter::new`.][7449]
- [Reflect: Added `FromReflect::take_from_reflect`][6566]
- [Reflect: Added `List::insert` and `List::remove`.][7063]
- [Reflect: Added `Map::remove`][6564]
- [Reflect: Added `ReflectFromReflect`][6245]
- [Reflect: Added `TypeRegistrationDeserializer`, which simplifies getting a `&TypeRegistration` while deserializing a string.][7094]
- [Reflect: Added methods to `List` that were previously provided by `Array`][7467]
- [Reflect: Added support for enums in reflection paths][6560]
- [Reflect: Added the `bevy_reflect_compile_fail_tests` crate for testing compilation errors][7041]
- [Reflect: bevy_reflect: Add missing primitive registrations][7815]
- [Reflect: impl `Reflect` for `&'static Path`][6755]
- [Reflect: implement `Reflect` for `Fxaa`][7527]
- [Reflect: implement `TypeUuid` for primitives and fix multiple-parameter generics having the same `TypeUuid`][6633]
- [Reflect: Implemented `Reflect` + `FromReflect` for window events and related types. These types are automatically registered when adding the `WindowPlugin`.][6235]
- [Reflect: Register Hash for glam types][6786]
- [Reflect: Register missing reflected types for `bevy_render`][6811]
- [Render: A pub field `extras` to `GltfNode`/`GltfMesh`/`GltfPrimitive` which store extras][6973]
- [Render: A pub field `material_extras` to `GltfPrimitive` which store material extras][6973]
- [Render: Add 'Color::as_lcha' function (#7757)][7766]
- [Render: Add `Camera::viewport_to_world_2d`][6557]
- [Render: Add a more familiar hex color entry][7060]
- [Render: add ambient lighting hook][5428]
- [Render: Add bevy logo to the lighting example to demo alpha mask shadows][7895]
- [Render: Add Box::from_corners method][6672]
- [Render: add OpenGL and DX11 backends][7481]
- [Render: Add orthographic camera support back to directional shadows][7796]
- [Render: add standard material depth bias to pipeline][7847]
- [Render: Add support for Rgb9e5Ufloat textures][6781]
- [Render: Added buffer usage field to buffers][7423]
- [Render: can define a value from inside a shader][7518]
- [Render: EnvironmentMapLight support for WebGL2][7737]
- [Render: Implement `ReadOnlySystemParam` for `Extract<>`][7182]
- [Render: Initial tonemapping options][7594]
- [Render: ShaderDefVal: add an `UInt` option][6881]
- [Render: Support raw buffers in AsBindGroup macro][7701]
- [Rendering: `Aabb` now implements `Copy`.][7401]
- [Rendering: `ExtractComponent` can specify output type, and outputting is optional.][6699]
- [Rendering: `Mssaa::samples`][7292]
- [Rendering: Add `#else ifdef` to shader preprocessing.][7431]
- [Rendering: Add a field `push_constant_ranges` to RenderPipelineDescriptor and ComputePipelineDescriptor][7681]
- [Rendering: Added  `Material::prepass_vertex_shader()` and `Material::prepass_fragment_shader()` to control the prepass from the `Material`][6284]
- [Rendering: Added `BloomSettings:lf_boost`, `BloomSettings:lf_boost_curvature`, `BloomSettings::high_pass_frequency` and `BloomSettings::composite_mode`.][6677]
- [Rendering: Added `BufferVec::extend`][6833]
- [Rendering: Added `BufferVec::truncate`][6833]
- [Rendering: Added `Camera::msaa_writeback` which can enable and disable msaa writeback.][7671]
- [Rendering: Added `CascadeShadowConfigBuilder` to help with creating `CascadeShadowConfig`][7456]
- [Rendering: Added `DepthPrepass` and `NormalPrepass` component to control which textures will be created by the prepass and available in later passes.][6284]
- [Rendering: Added `Draw<T>::prepare` optional trait function.][6885]
- [Rendering: Added `DrawFunctionsInternals::id()`][6745]
- [Rendering: Added `FallbackImageCubemap`.][7051]
- [Rendering: Added `FogFalloff` enum for selecting between three widely used “traditional” fog falloff modes: `Linear`, `Exponential` and `ExponentialSquared`, as well as a more advanced `Atmospheric` fog;][6412]
- [Rendering: Added `get_input_node`][6720]
- [Rendering: Added `Lcha` member to `bevy_render::color::Color` enum][7483]
- [Rendering: Added `MainTaret::main_texture_other`][7343]
- [Rendering: Added `PhaseItem::entity`][6885]
- [Rendering: Added `prepass_enabled` flag to the `MaterialPlugin` that will control if a material uses the prepass or not.][6284]
- [Rendering: Added `prepass_enabled` flag to the `PbrPlugin` to control if the StandardMaterial uses the prepass. Currently defaults to false.][6284]
- [Rendering: Added `PrepassNode` that runs before the main pass][6284]
- [Rendering: Added `PrepassPlugin` to extract/prepare/queue the necessary data][6284]
- [Rendering: Added `RenderCommand::ItemorldQuery` associated type.][6885]
- [Rendering: Added `RenderCommand::ViewWorldQuery` associated type.][6885]
- [Rendering: Added `RenderContext::add_command_buffer`][7248]
- [Rendering: Added `RenderContext::begin_tracked_render_pass`.][7053]
- [Rendering: Added `RenderContext::finish`][7248]
- [Rendering: Added `RenderContext::new`][7248]
- [Rendering: Added `SortedCameras`, exposing information that was previously internal to the camera driver node.][7671]
- [Rendering: Added `try_add_node_edge`][6720]
- [Rendering: Added `try_add_slot_edge`][6720]
- [Rendering: Added `with_r`, `with_g`, `with_b`, and `with_a` to `Color`.][6899]
- [Rendering: Added 2x and 8x sample counts for MSAA.][7684]
- [Rendering: Added a `#[storage(index)]` attribute to the derive `AsBindGroup` macro.][6129]
- [Rendering: Added an `EnvironmentMapLight` camera component that adds additional ambient light to a scene.][7051]
- [Rendering: Added argument to `ScalingMode::WindowSize` that specifies the number of pixels that equals one world unit.][6201]
- [Rendering: Added cylinder shape][6809]
- [Rendering: Added example `shaders/texture_binding_array`.][6995]
- [Rendering: Added new capabilities for shader validation.][6995]
- [Rendering: Added specializable `BlitPipeline` and ported the upscaling node to use this.][7671]
- [Rendering: Added subdivisions field to shape::Plane][7546]
- [Rendering: Added support for additive and multiplicative blend modes in the PBR `StandardMaterial`, via `AlphaMode::Add` and `AlphaMode::Multiply`;][6644]
- [Rendering: Added support for distance-based fog effects for PBR materials, controllable per-camera via the new `FogSettings` component;][6412]
- [Rendering: Added support for KTX2 `R8_SRGB`, `R8_UNORM`, `R8G8_SRGB`, `R8G8_UNORM`, `R8G8B8_SRGB`, `R8G8B8_UNORM` formats by converting to supported wgpu formats as appropriate][4594]
- [Rendering: Added support for premultiplied alpha in the PBR `StandardMaterial`, via `AlphaMode::Premultiplied`;][6644]
- [Rendering: Added the ability to `#[derive(ExtractComponent)]` with an optional filter.][7399]
- [Rendering: Added: `bevy_render::color::LchRepresentation` struct][7483]
- [Rendering: Clone impl for MaterialPipeline][7548]
- [Rendering: Implemented `Clone` for all pipeline types.][6653]
- [Rendering: Smooth Transition between Animations][6922]
- [Support optional env variable `BEVY_ASSET_ROOT` to explicitly specify root assets directory.][5346]
- [Task: Add thread create/destroy callbacks to TaskPool][6561]
- [Tasks: Added `ThreadExecutor` that can only be ticked on one thread.][7087]
- [the extension methods `in_schedule(label)` and  `on_startup()` for configuring the schedule a system belongs to.][7790]
- [Transform: Added `GlobalTransform::reparented_to`][7020]
- [UI: `Size::new` is now `const`][6602]
- [UI: Add const to methods and const defaults to bevy_ui][5542]
- [UI: Added `all`, `width` and `height` functions to `Size`.][7468]
- [UI: Added `Anchor` component to `Text2dBundle`][6807]
- [UI: Added `CalculatedSize::preserve_aspect_ratio`][6825]
- [UI: Added `Component` derive to `Anchor`][6807]
- [UI: Added `RelativeCursorPosition`, and an example showcasing it][7199]
- [UI: Added `Text::with_linebreak_behaviour`][7283]
- [UI: Added `TextBundle::with_linebreak_behaviour`][7283]
- [UI: Added a `BackgroundColor` component to `TextBundle`.][7596]
- [UI: Added a helper method `with_background_color` to `TextBundle`.][7596]
- [UI: Added the `SpaceEvenly` variant to `AlignContent`.][7859]
- [UI: Added the `Start` and `End` variants to `AlignItems`, `AlignSelf`, `AlignContent` and `JustifyContent`.][7859]
- [UI: Adds `flip_x` and `flip_y` fields to `ExtractedUiNode`.][6292]
- [Utils: Added `SyncCell::read`, which allows shared access to values that already implement the `Sync` trait.][7718]
- [Utils: Added the guard type `bevy_utils::OnDrop`.][7181]
- [Window: Add `Windows::get_focused(_mut)`][6571]
- [Window: add span to winit event handler][6612]
- [Window: Transparent window on macos][7617]
- [Windowing: `WindowDescriptor` renamed to `Window`.][5589]
- [Windowing: Added `hittest` to `WindowAttributes`][6664]
- [Windowing: Added `Window::prevent_default_event_handling` . This allows bevy apps to not override default browser behavior on hotkeys like F5, F12, Ctrl+R etc.][7304]
- [Windowing: Added `WindowDescriptor.always_on_top` which configures a window to stay on top.][6527]
- [Windowing: Added an example `cargo run --example fallthrough`][6664]
- [Windowing: Added the `hittest`’s setters/getters][6664]
- [Windowing: Modifed the `WindowDescriptor`’s `Default` impl.][6664]
- [Windowing: Modified the `WindowBuilder`][6664]

## Changed

- [Animation: `AnimationPlayer` that are on a child or descendant of another entity with another player will no longer be run.][6785]
- [Animation: Animation sampling now runs fully multi-threaded using threads from `ComputeTaskPool`.][6785]
- [App: Adapt path type of dynamically_load_plugin][6734]
- [App: Break CorePlugin into TaskPoolPlugin, TypeRegistrationPlugin, FrameCountPlugin.][7083]
- [App: Increment FrameCount in CoreStage::Last.][7477]
- [App::run() will now panic when called from Plugin::build()][4241]
- [Asset: `AssetIo::watch_path_for_changes` allows watched path and path to reload to differ][6797]
- [Asset: make HandleUntyped::id private][7076]
- [Audio: `AudioOutput` is now a `Resource`. It's no longer `!Send`][6436]
- [Audio: AudioOutput is actually a normal resource now, not a non-send resource][7262]
- [ECS: `.label(SystemLabel)` is now referred to as `.in_set(SystemSet)`][7267]
- [ECS: `App::add_default_labels` is now `App::add_default_sets`][7267]
- [ECS: `App::add_system_set` was renamed to `App::add_systems`][7267]
- [ECS: `Archetype` indices and `Table` rows have been newtyped as `ArchetypeRow` and `TableRow`.][4878]
- [ECS: `ArchetypeGeneration` now implements `Ord` and `PartialOrd`.][6742]
- [ECS: `bevy_pbr::add_clusters` is no longer an exclusive system][7267]
- [ECS: `Bundle::get_components` now takes a `FnMut(StorageType, OwningPtr)`. The provided storage type must be correct for the component being fetched.][6902]
- [ECS: `ChangeTrackers<T>` has been deprecated. It will be removed in Bevy 0.11.][7306]
- [ECS: `Command` closures no longer need to implement the marker trait `std::marker::Sync`.][7014]
- [ECS: `CoreStage` and `StartupStage` enums are now `CoreSet` and `StartupSet`][7267]
- [ECS: `EntityMut::world_scope` now allows returning a value from the immediately-computed closure.][7385]
- [ECS: `EntityMut`: rename `remove_intersection` to `remove` and `remove` to `take`][7810]
- [ECS: `EventReader::clear` now takes a mutable reference instead of consuming the event reader.][6851]
- [ECS: `EventWriter::send_batch` will only log a TRACE level log if the batch is non-empty.][7753]
- [ECS: `oldest_id` and `get_event` convenience methods added to `Events<T>`.][5735]
- [ECS: `OwningPtr::drop_as` will now panic in debug builds if the pointer is not aligned.][7117]
- [ECS: `OwningPtr::read` will now panic in debug builds if the pointer is not aligned.][7117]
- [ECS: `Ptr::deref` will now panic in debug builds if the pointer is not aligned.][7117]
- [ECS: `PtrMut::deref_mut` will now panic in debug builds if the pointer is not aligned.][7117]
- [ECS: `Query::par_for_each(_mut)` has been changed to `Query::par_iter(_mut)` and will now automatically try to produce a batch size for callers based on the current `World` state.][4777]
- [ECS: `RemovedComponents` now internally uses an `Events<RemovedComponentsEntity>` instead of an `Events<Entity>`][7503]
- [ECS: `SceneSpawnerSystem` now runs under `CoreSet::Update`, rather than `CoreStage::PreUpdate.at_end()`.][7267]
- [ECS: `StartupSet` is now a base set][7574]
- [ECS: `System::default_labels` is now `System::default_system_sets`.][7267]
- [ECS: `SystemLabel` trait was replaced by `SystemSet`][7267]
- [ECS: `SystemParamState::apply` now takes a `&SystemMeta` parameter in addition to the provided `&mut World`.][6900]
- [ECS: `SystemTypeIdLabel<T>` was replaced by `SystemSetType<T>`][7267]
- [ECS: `tick_global_task_pools_on_main_thread` is no longer run as an exclusive system. Instead, it has been replaced by `tick_global_task_pools`, which uses a `NonSend` resource to force running on the main thread.][7267]
- [ECS: `Tick::is_older_than` was renamed to `Tick::is_newer_than`. This is not a functional change, since that was what was always being calculated, despite the wrong name.][7561]
- [ECS: `UnsafeWorldCell::world` is now used to get immutable access to the whole world instead of just the metadata which can now be done via `UnsafeWorldCell::world_metadata`][7381]
- [ECS: `World::init_non_send_resource` now returns the generated `ComponentId`.][7284]
- [ECS: `World::init_resource` now returns the generated `ComponentId`.][7284]
- [ECS: `World::iter_entities` now returns an iterator of `EntityRef` instead of `Entity`.][6843]
- [ECS: `World`s can now only hold a maximum of 2^32 - 1 tables.][6681]
- [ECS: `World`s can now only hold a maximum of 2^32- 1 archetypes.][6681]
- [ECS: `WorldId` now implements `SystemParam` and will return the id of the world the system is running in][7741]
- [ECS: Adding rendering extraction systems now panics rather than silently failing if no subapp with the `RenderApp` label is found.][7267]
- [ECS: Allow adding systems to multiple sets that share the same base set][7709]
- [ECS: change `is_system_type() -> bool` to `system_type() -> Option<TypeId>`][7715]
- [ECS: changed some `UnsafeWorldCell` methods to take `self` instead of `&self`/`&mut self` since there is literally no point to them doing that][7381]
- [ECS: Changed: `Query::for_each(_mut)`, `QueryParIter` will now leverage autovectorization to speed up query iteration where possible.][6547]
- [ECS: Default to using ExecutorKind::SingleThreaded on wasm32][7717]
- [ECS: Ensure `Query` does not use the wrong `World`][7150]
- [ECS: Exclusive systems may now be used with system piping.][7023]
- [ECS: expose `ScheduleGraph` for use in third party tools][7522]
- [ECS: extract topsort logic to a new method, one pass to detect cycles and …][7727]
- [ECS: Fixed time steps now use a schedule (`CoreSchedule::FixedTimeStep`) rather than a run criteria.][7267]
- [ECS: for disconnected, use Vec instead of HashSet to reduce insert overhead][7744]
- [ECS: Implement `SparseSetIndex` for `WorldId`][7125]
- [ECS: Improve the panic message for schedule build errors][7860]
- [ECS: Lift the 16-field limit from the `SystemParam` derive][6867]
- [ECS: Make `EntityRef::new` unsafe][7222]
- [ECS: Make `Query` fields private][7149]
- [ECS: make `ScheduleGraph::initialize` public][7723]
- [ECS: Make boxed conditions read-only][7786]
- [ECS: Make RemovedComponents mirror EventReaders api surface][7713]
- [ECS: Mark TableRow and TableId as repr(transparent)][7166]
- [ECS: Most APIs returning `&UnsafeCell<ComponentTicks>` now returns `TickCells` instead, which contains two separate `&UnsafeCell<Tick>` for either component ticks.][6547]
- [ECS: Move MainThreadExecutor for stageless migration.][7444]
- [ECS: Move safe operations out of `unsafe` blocks in `Query`][7851]
- [ECS: Optimize `.nth()` and `.last()` for event iterators][7530]
- [ECS: Optimize `Iterator::count` for event iterators][7582]
- [ECS: Provide public `EntityRef::get_change_ticks_by_id` that takes `ComponentId`][6683]
- [ECS: refactor: move internals from `entity_ref` to `World`, add `SAFETY` comments][6402]
- [ECS: Rename `EntityId` to `EntityIndex`][6732]
- [ECS: Rename `UnsafeWorldCellEntityRef` to `UnsafeEntityCell`][7568]
- [ECS: Rename schedule v3 to schedule][7519]
- [ECS: Rename state_equals condition to in_state][7677]
- [ECS: Replace `World::read_change_ticks` with `World::change_ticks` within `bevy_ecs` crate][6816]
- [ECS: Replaced the trait `ReadOnlySystemParamFetch` with `ReadOnlySystemParam`.][6865]
- [ECS: Simplified the `SystemParamFunction` and `ExclusiveSystemParamFunction` traits.][7675]
- [ECS: Speed up `CommandQueue` by storing commands more densely][6391]
- [ECS: Stageless: move final apply outside of spawned executor][7445]
- [ECS: Stageless: prettier cycle reporting][7463]
- [ECS: Systems without `Commands` and  `ParallelCommands` will no longer show a `system_commands` span when profiling.][6900]
- [ECS: The `ReportHierarchyIssue` resource now has a public constructor (`new`), and implements `PartialEq`][7267]
- [ECS: The `StartupSchedule` label is now defined as part of the `CoreSchedules` enum][7267]
- [ECS: The `SystemParam` derive is now more flexible, allowing you to omit unused lifetime parameters.][6694]
- [ECS: the top level `bevy_ecs::schedule` module was replaced with `bevy_ecs::scheduling`][7267]
- [ECS: Use `World` helper methods for sending `HierarchyEvent`s][6921]
- [ECS: Use a bounded channel in the multithreaded executor][7829]
- [ECS: Use a default implementation for `set_if_neq`][7660]
- [ECS: Use consistent names for marker generics][7788]
- [ECS: Use correct terminology for a `NonSend` run condition panic][7841]
- [ECS: Use default-implemented methods for `IntoSystemConfig<>`][7870]
- [ECS: use try_send to replace send.await, unbounded channel should always b…][7745]
- [General: The MSRV of the engine is now 1.67.][7379]
- [Input: Bump gilrs version to 0.10][6558]
- [IOS, Android... same thing][7493]
- [Math: Update `glam` to `0.23`][7883]
- [Math: use `Mul<f32>` to double the value of `Vec3`][6607]
- [Reflect: bevy_reflect now uses a fixed state for its hasher, which means the output of `Reflect::reflect_hash` is now deterministic across processes.][7583]
- [Reflect: Changed function signatures of `ReflectComponent` methods, `apply`, `remove`, `contains`, and `reflect`.][7206]
- [Reflect: Changed the `List::push` and `List::pop` to have default implementations.][7063]
- [Reflect: Registered `SmallVec<[Entity; 8]>` in the type registry][6578]
- [Renamed methods on `GetPath`:][7321]
  - `path` -> `reflect_path`
  - `path_mut` -> `reflect_path_mut`
  - `get_path` -> `path`
  - `get_path_mut` -> `path_mut`
- [Render: Allow prepass in webgl][7537]
- [Render: bevy_pbr: Avoid copying structs and using registers in shaders][7069]
- [Render: bevy_pbr: Clear fog DynamicUniformBuffer before populating each frame][7432]
- [Render: bevy_render: Run calculate_bounds in the end-of-update exclusive systems][7127]
- [Render: Change the glTF loader to use `Camera3dBundle`][7890]
- [Render: Changed &mut PipelineCache to &PipelineCache][7598]
- [Render: Intepret glTF colors as linear instead of sRGB][6828]
- [Render: Move 'startup' Resource `WgpuSettings`  into the `RenderPlugin`][6946]
- [Render: Move prepass functions to prepass_utils][7354]
- [Render: Only compute sprite color once per quad][7498]
- [Render: Only execute `#define` if current scope is accepting lines][7798]
- [Render: Pipelined Rendering][6503]
- [Render: Refactor Globals and View structs into separate shaders][7512]
- [Render: Replace UUID based IDs with a atomic-counted ones][6988]
- [Render: run clear trackers on render world][6878]
- [Render: set cull mode: None for Mesh2d][7514]
- [Render: Shader defs can now have a value][5900]
- [Render: Shrink ComputedVisibility][6305]
- [Render: Use prepass shaders for shadows][7784]
- [Rendering: `add_node_edge` is now infallible (panics on error)][6720]
- [Rendering: `add_slot_edge` is now infallible (panics on error)][6720]
- [Rendering: `AsBindGroup` is now object-safe.][6937]
- [Rendering: `BloomSettings::knee` renamed to `BloomPrefilterSettings::softness`.][6677]
- [Rendering: `BloomSettings::threshold` renamed to `BloomPrefilterSettings::threshold`.][6677]
- [Rendering: `HexColorError::Hex` has been renamed to `HexColorError::Char`][6940]
- [Rendering: `input_node` now panics on `None`][6720]
- [Rendering: `ktx2` and `zstd` are now part of bevy’s default enabled features][7696]
- [Rendering: `Msaa` is now enum][7292]
- [Rendering: `PipelineCache` no longer requires mutable access in order to queue render / compute pipelines.][7205]
- [Rendering: `RenderContext::command_encoder` is now private. Use the accessor `RenderContext::command_encoder()` instead.][7248]
- [Rendering: `RenderContext::render_device` is now private. Use the accessor `RenderContext::render_device()` instead.][7248]
- [Rendering: `RenderContext` now supports adding external `CommandBuffer`s for inclusion into the render graphs. These buffers can be encoded outside of the render graph (i.e. in a system).][7248]
- [Rendering: `scale` is now applied before updating `area`. Reading from it will take `scale` into account.][6201]
- [Rendering: `SkinnedMeshJoints::build` now takes a `&mut BufferVec` instead of a `&mut Vec` as a parameter.][6833]
- [Rendering: `StandardMaterial` now defaults to a dielectric material (0.0 `metallic`) with 0.5 `perceptual_roughness`.][7664]
- [Rendering: `TrackedRenderPass` now requires a `&RenderDevice` on construction.][7053]
- [Rendering: `Visibility` is now an enum][6320]
- [Rendering: Bloom now looks different.][6677]
- [Rendering: Directional lights now use cascaded shadow maps for improved shadow quality.][7064]
- [Rendering: ExtractedMaterials, extract_materials and prepare_materials are now public][7548]
- [Rendering: For performance reasons, some detailed renderer trace logs now require the use of cargo feature `detailed_trace` in addition to setting the log level to `TRACE` in order to be shown.][7639]
- [Rendering: Made cameras with the same target share the same `main_texture` tracker, which ensures continuity across cameras.][7671]
- [Rendering: Renamed `ScalingMode::Auto` to `ScalingMode::AutoMin`.][6496]
- [Rendering: Renamed `ScalingMode::None` to `ScalingMode::Fixed`][6201]
- [Rendering: Renamed `window_origin` to `viewport_origin`][6201]
- [Rendering: Renamed the `priority` field on `Camera` to `order`.][6908]
- [Rendering: Replaced `left`, `right`, `bottom`, and `top` fields with a single `area: Rect`][6201]
- [Rendering: StandardMaterials will now appear brighter and more saturated at high roughness, due to internal material changes. This is more physically correct.][7051]
- [Rendering: The `layout` field of `RenderPipelineDescriptor` and `ComputePipelineDescriptor` is now mandatory.][7681]
- [Rendering: The `rangefinder` module has been moved into the `render_phase` module.][7016]
- [Rendering: The bloom example has been renamed to bloom_3d and improved. A bloom_2d example was added.][6677]
- [Rendering: the SubApp Extract stage has been separated from running the sub app schedule.][7046]
- [Rendering: To enable multiple `RenderPhases` to share the same `TrackedRenderPass`, the `RenderPhase::render` signature has changed.][7043]
- [Rendering: update its `Transform` in order to preserve its `GlobalTransform` after the parent change][7024]
- [Rendering: Updated to wgpu 0.15, wgpu-hal 0.15.1, and naga 0.11][7356]
- [Rendering: Users can now use the DirectX Shader Compiler (DXC) on Windows with DX12 for faster shader compilation and ShaderModel 6.0+ support (requires `dxcompiler.dll` and `dxil.dll`)][7356]
- [Rendering: You can now set up the rendering code of a `RenderPhase` directly using the `RenderPhase::render` method, instead of implementing it manually in your render graph node.][7013]
- [Scenes: `SceneSpawner::spawn_dynamic` now returns `InstanceId` instead of `()`.][6663]
- [Shape: Change `From<Icosphere>` to `TryFrom<Icosphere>`][6484]
- [Tasks: `Scope` now uses `FallibleTask` to await the cancellation of all remaining tasks when it’s dropped.][6696]
- [Time: `Time::set_relative_speed_fXX` now allows a relative speed of -0.0.][7740]
- [UI: `FocusPolicy` default has changed from `FocusPolicy::Block` to `FocusPolicy::Pass`][7161]
- [UI: `TextPipeline::queue_text` and `GlyphBrush::compute_glyphs` now need a TextLineBreakBehaviour argument, in order to pass through the new field.][7283]
- [UI: `update_image_calculated_size_system` sets `preserve_aspect_ratio` to true for nodes with images.][6825]
- [UI: Added `Changed<Node>` to the change detection query of `text_system`. This ensures that any change in the size of a text node will cause any text it contains to be recomputed.][7674]
- [UI: Changed `Size::height` so it sets the `width` to `Val::AUTO`.][7626]
- [UI: Changed `Size::width` so it sets the `height` to `Val::AUTO`.][7626]
- [UI: Changed `TextAlignment` into an enum with `Left`, `Center`, and `Right` variants.][6807]
- [UI: Changed extract_uinodes to extract the flip_x and flip_y values from UiImage.][6292]
- [UI: Changed prepare_uinodes to swap the UV coordinates as required.][6292]
- [UI: Changed Taffy version to 0.3.3 and disabled its `grid` feature.][7859]
- [UI: Changed the `Size` `width` and `height` default values to `Val::Auto`][7475]
- [UI: Changed the `size` field of `CalculatedSize` to a Vec2.][7641]
- [UI: Changed UiImage derefs to texture field accesses.][6292]
- [UI: Changed UiImage to a struct with texture, flip_x, and flip_y fields.][6292]
- [UI: Modified the `text2d` example to show both linebreaking behaviours.][7283]
- [UI: Renamed `image_node_system` to `update_image_calculated_size_system`][6674]
- [UI: Renamed the `background_color` field of `ExtractedUiNode` to `color`.][7452]
- [UI: Simplified the UI examples. Replaced numeric values with the Flex property enums or elided them where possible, and removed the remaining use of auto margins.][7626]
- [UI: The `MeasureFunc` only preserves the aspect ratio when `preserve_aspect_ratio` is true.][6825]
- [UI: Updated `from_style` for Taffy 0.3.3.][7859]
- [UI: Upgraded to Taffy 0.2, improving UI layout performance significantly and adding the flexbox `gap` property and `AlignContent::SpaceEvenly`.][6743]
- [UI: Use `f32::INFINITY` instead of `f32::MAX` to represent unbounded text in Text2dBounds][6807]
- [Window: expose cursor position with scale][7297]
- [Window: Make WindowId::primary() const][6582]
- [Window: revert stage changed for window closing][7296]
- [Windowing: `WindowId` is now `Entity`.][5589]
- [Windowing: Moved `changed_window` and `despawn_window` systems to `CoreStage::Last` to avoid systems making changes to the `Window` between `changed_window` and the end of the frame as they would be ignored.][7517]
- [Windowing: Requesting maximization/minimization is done on the [`Window::state`] field.][5589]
- [Windowing: Width/height consolidated into a `WindowResolution` component.][5589]

## Removed

- [App: Removed `App::add_sub_app`][7290]
- [App: Rename dynamic feature][7340]
- [ECS: Remove .on_update method to improve API consistency and clarity][7667]
- [ECS: Remove `BuildWorldChildren` impl from `WorldChildBuilder`][6727]
- [ECS: Remove a duplicate lookup in `apply_state_transitions`][7800]
- [ECS: Remove an incorrect impl of `ReadOnlySystemParam` for `NonSendMut`][7243]
- [ECS: Remove APIs deprecated in 0.9][6801]
- [ECS: Remove broken `DoubleEndedIterator` impls on event iterators][7469]
- [ECS: Remove duplicate lookups from `Resource` initialization][7174]
- [ECS: Remove useless access to archetype in `UnsafeWorldCell::fetch_table`][7665]
- [ECS: Removed `AddBundle`. `Edges::get_add_bundle` now returns `Option<ArchetypeId>`][6742]
- [ECS: Removed `Archetype::new` and `Archetype::is_empty`.][6742]
- [ECS: Removed `ArchetypeComponentId::new` and `ArchetypeComponentId::value`.][6742]
- [ECS: Removed `ArchetypeGeneration::value`][6742]
- [ECS: Removed `ArchetypeId::new` and `ArchetypeId::value`.][6742]
- [ECS: Removed `ArchetypeIdentity`.][6742]
- [ECS: Removed `Archetypes`’s `Default` implementation.][6742]
- [ECS: Removed `AsSystemLabel` trait][7267]
- [ECS: Removed `Entities::alloc_at_without_replacement` and `AllocAtWithoutReplacement`.][6740]
- [ECS: Removed `Entities`’s `Default` implementation.][6740]
- [ECS: Removed `EntityMeta`][6740]
- [ECS: Removed `on_hierarchy_reports_enabled` run criteria (now just uses an ad hoc resource checking run condition)][7267]
- [ECS: Removed `RunCriteriaLabel`][7267]
- [ECS: Removed `RunCriteriaLabel`][7267]
- [ECS: Removed `SystemParamFetch`, its functionality has been moved to `SystemParamState`.][6865]
- [ECS: Removed `Table::component_capacity`][4928]
- [ECS: Removed `transform_propagate_system_set`: this was a nonstandard pattern that didn’t actually provide enough control. The systems are already `pub`: the docs have been updated to ensure that the third-party usage is clear.][7267]
- [ECS: removed `UnsafeWorldCell::storages` since that is probably unsound since storages contains the actual component/resource data not just metadata][7381]
- [ECS: Removed stages, and all code that mentions stages][7267]
- [ECS: Removed states have been dramatically simplified, and no longer use a stack][7267]
- [ECS: Removed systems in `RenderSet/Stage::Extract` no longer warn when they do not read data from the main world][7267]
- [ECS: Removed the bound `T: Sync` from `Local<T>` when used as an `ExclusiveSystemParam`.][7040]
- [ECS: Removed the method `ExclusiveSystemParamState::apply`.][7489]
- [ECS: Removed the trait `ExclusiveSystemParamState`, merging its functionality into `ExclusiveSystemParam`.][6919]
- [ECS: Removed the trait `SystemParamState`, merging its functionality into `SystemParam`.][6919]
- [ECS: Support `SystemParam` types with const generics][7001]
- [ECS: Use T::Storage::STORAGE_TYPE to optimize out unused branches][6800]
- [Hierarchy: Expose transform propagate systems][7145]
- [Hierarchy: Make adding children idempotent][6763]
- [Hierarchy: Remove `EntityCommands::add_children`][6942]
- [Input: Gamepad events refactor][6965]
- [Reflect: Make proc macros hygienic in bevy_reflect_derive][6752]
- [Reflect: Removed `#[module]` helper attribute for `Reflect` derives (this is not currently used)][7148]
- [Reflect: Removed `Array` as supertrait of `List`][7467]
- [Reflect: Removed `PixelInfo` and get `pixel_size` from wgpu][6820]
- [Reflect: Removed `ReflectSerialize` and `ReflectDeserialize` registrations from most glam types][6580]
- [Remove unnecessary `Default` impl of HandleType][7472]
- [Remove warning about missed events due to false positives][6730]
- [Render: Make Core Pipeline Graph Nodes Public][6605]
- [Render: Optimize color computation in prepare_uinodes][7311]
- [Render: Organized scene_viewer into plugins for reuse and organization][6936]
- [Render: put `update_frusta::<Projection>` in `UpdateProjectionFrusta` set][7526]
- [Render: Remove dependency on the mesh struct in the pbr function][7597]
- [Render: remove potential ub in render_resource_wrapper][7279]
- [Render: Remove redundant bitwise OR `TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES`][7033]
- [Render: Remove the early exit to make sure the prepass textures are cleared][7891]
- [Render: remove the image loaded check for nodes without images in extract_uinodes][7280]
- [Render: Remove unnecessary alternate create_texture path in prepare_asset for Image][6671]
- [Render: remove unused var in fxaa shader][7509]
- [Render: set AVAILABLE_STORAGE_BUFFER_BINDINGS to the actual number of buffers available][6787]
- [Render: Use `Time` `resource` instead of `Extract`ing `Time`][7316]
- [Render: use better set inheritance in render systems][7524]
- [Render: use blendstate blend for alphamode::blend][7899]
- [Render: Use Image::default for 1 pixel white texture directly][7884]
- [Rendering: Removed `bevy_render::render_phase::DrawState`. It was not usable in any form outside of `bevy_render`.][7053]
- [Rendering: Removed `BloomSettings::scale`.][6677]
- [Rendering: Removed `EntityPhaseItem` trait][6885]
- [Rendering: Removed `ExtractedJoints`.][6833]
- [Rendering: Removed `SetShadowViewBindGroup`, `queue_shadow_view_bind_group()`, and `LightMeta::shadow_view_bind_group` in favor of reusing the prepass view bind group.][7875]
- [Rendering: Removed the `render` feature group.][6912]
- [Scene: scene viewer: can select a scene from the asset path][6859]
- [Text: Warn instead of erroring when max_font_atlases is exceeded][6673]
- [Transform: Removed `GlobalTransform::translation_mut`][7134]
- [UI: Re-enable taffy send+sync assert][7769]
- [UI: Remove `TextError::ExceedMaxTextAtlases(usize)` variant][6796]
- [UI: Remove needless manual default impl of ButtonBundle][6970]
- [UI: Removed `HorizontalAlign` and `VerticalAlign`.][6807]
- [UI: Removed `ImageMode`.][6674]
- [UI: Removed `QueuedText`][7414]
- [UI: Removed the `image_mode` field from `ImageBundle`][6674]
- [UI: Removed the `Val` <-> `f32`  conversion for  `CalculatedSize`.][7641]
- [Update toml_edit to 0.18][7370]
- [Update tracing-chrome requirement from 0.6.0 to 0.7.0][6709]
- [Window: Remove unnecessary windows.rs file][7277]
- [Windowing: `window.always_on_top` has been removed, you can now use `window.window_level`][7480]
- [Windowing: Removed `ModifiesWindows` system label.][7517]

## Fixed

- [Asset: Fix asset_debug_server hang. There should be at most one ThreadExecut…][7825]
- [Asset: fix load_internal_binary_asset with debug_asset_server][7246]
- [Assets: Hot reloading for `LoadContext::read_asset_bytes`][6797]
- [Diagnostics: Console log messages now show when the `trace_tracy` feature was enabled.][6955]
- [ECS: Fix `last_changed()` and `set_last_changed()` for `MutUntyped`][7619]
- [ECS: Fix a miscompilation with `#[derive(SystemParam)]`][7105]
- [ECS: Fix get_unchecked_manual using archetype index instead of table row.][6625]
- [ECS: Fix ignored lifetimes in `#[derive(SystemParam)]`][7458]
- [ECS: Fix init_non_send_resource overwriting previous values][7261]
- [ECS: fix mutable aliases for a very short time if `WorldCell` is already borrowed][6639]
- [ECS: Fix partially consumed `QueryIter` and `QueryCombinationIter` having invalid `size_hint`][5214]
- [ECS: Fix PipeSystem panicking with exclusive systems][6698]
- [ECS: Fix soundness bug with `World: Send`. Dropping a `World` that contains a `!Send` resource on the wrong thread will now panic.][6534]
- [ECS: Fix Sparse Change Detection][6896]
- [ECS: Fix trait bounds for run conditions][7688]
- [ECS: Fix unsoundnes in `insert` `remove` and `despawn`][7805]
- [ECS: Fix unsoundness in `EntityMut::world_scope`][7387]
- [ECS: Fixed `DetectChanges::last_changed` returning the wrong value.][7560]
- [ECS: Fixed `DetectChangesMut::set_last_changed` not actually updating the `changed` tick.][7560]
- [ECS: Fixed `Res` and `Query` parameter never being mutually exclusive.][5105]
- [ECS: Fixed a bug that caused `#[derive(SystemParam)]` to leak the types of private fields.][7056]
- [ECS: schedule_v3: fix default set for systems not being applied][7350]
- [ECS: Stageless: close the finish channel so executor doesn't deadlock][7448]
- [ECS: Stageless: fix unapplied systems][7446]
- [Hierarchy: don't error when sending HierarchyEvents when Event type not registered][7031]
- [Hierarchy: Fix unsoundness for `propagate_recursive`][7003]
- [Hierarchy: Fixed missing `ChildAdded` events][6926]
- [Input: Avoid triggering change detection for inputs][6847]
- [Input: Fix `AxisSettings::new` only accepting invalid bounds][7233]
- [Input: Fix incorrect behavior of `just_pressed` and `just_released` in `Input<GamepadButton>`][7238]
- [Input: Removed Mobile Touch event y-axis flip][6597]
- [Reflect: bevy_reflect: Fix misplaced impls][6829]
- [Reflect: Fix bug where deserializing unit structs would fail for non-self-describing formats][6722]
- [Reflect: Fix bug where scene deserialization using certain readers could fail (e.g. `BufReader`, `File`, etc.)][6894]
- [Reflect: fix typo in bevy_reflect::impls::std GetTypeRegistration for vec like…][7520]
- [Reflect: Retain `::` after `>`, `)` or bracket when shortening type names][7755]
- [Render: bevy_core_pipeline: Fix prepass sort orders][7539]
- [Render: Cam scale cluster fix][7078]
- [Render: fix ambiguities in render schedule][7725]
- [Render: fix bloom viewport][6802]
- [Render: Fix dependency of shadow mapping on the optional `PrepassPlugin`][7878]
- [Render: Fix feature gating in texture_binding_array example][7425]
- [Render: Fix material alpha_mode in example global_vs_local_translation][6658]
- [Render: fix regex for shader define: must have at least one whitespace][7754]
- [Render: fix shader_instancing][7305]
- [Render: fix spot dir nan again][7176]
- [Render: Recreate tonemapping bind group if view uniforms buffer has changed][7904]
- [Render: Shadow render phase - pass the correct view entity][7048]
- [Render: Text2d doesn't recompute text on changes to the text's bounds][7846]
- [Render: wasm: pad globals uniform also in 2d][6643]
- [Rendering: Emission strength is now correctly interpreted by the `StandardMaterial` as linear instead of sRGB.][7897]
- [Rendering: Fix deband dithering intensity for non-HDR pipelines.][6707]
- [Rendering: Fixed StandardMaterial occlusion being incorrectly applied to direct lighting.][7051]
- [Rendering: Fixed the alpha channel of the `image::DynamicImage::ImageRgb32F` to `bevy_render::texture::Image` conversion in `bevy_render::texture::Image::from_dynamic()`.][6914]
- [Scene: Cleanup dynamic scene before building][6254]
- [Task: Fix panicking on another scope][6524]
- [UI: `Size::height` sets `width` not `height`][7478]
- [UI: Don't ignore UI scale for text][7510]
- [UI: Fix `bevy_ui` compile error without `bevy_text`][7877]
- [UI: Fix overflow scaling for images][7142]
- [UI: fix upsert_leaf not setting a MeasureFunc for new leaf nodes][7351]
- [Window: Apply `WindowDescriptor` settings in all modes][6934]
- [Window: break feedback loop when moving cursor][7298]
- [Window: create window as soon as possible][7668]
- [Window: Fix a typo on `Window::set_minimized`][7276]
- [Window: Fix closing window does not exit app in desktop_app mode][7628]
- [Window: fix cursor grab issue][7010]
- [Window: Fix set_cursor_grab_mode to try an alternative mode before giving an error][6599]

[3212]: https://github.com/bevyengine/bevy/pull/3212
[4241]: https://github.com/bevyengine/bevy/pull/4241
[4594]: https://github.com/bevyengine/bevy/pull/4594
[4777]: https://github.com/bevyengine/bevy/pull/4777
[4878]: https://github.com/bevyengine/bevy/pull/4878
[4928]: https://github.com/bevyengine/bevy/pull/4928
[5105]: https://github.com/bevyengine/bevy/pull/5105
[5214]: https://github.com/bevyengine/bevy/pull/5214
[5346]: https://github.com/bevyengine/bevy/pull/5346
[5400]: https://github.com/bevyengine/bevy/pull/5400
[5428]: https://github.com/bevyengine/bevy/pull/5428
[5454]: https://github.com/bevyengine/bevy/pull/5454
[5542]: https://github.com/bevyengine/bevy/pull/5542
[5589]: https://github.com/bevyengine/bevy/pull/5589
[5735]: https://github.com/bevyengine/bevy/pull/5735
[5900]: https://github.com/bevyengine/bevy/pull/5900
[6028]: https://github.com/bevyengine/bevy/pull/6028
[6035]: https://github.com/bevyengine/bevy/pull/6035
[6129]: https://github.com/bevyengine/bevy/pull/6129
[6201]: https://github.com/bevyengine/bevy/pull/6201
[6235]: https://github.com/bevyengine/bevy/pull/6235
[6245]: https://github.com/bevyengine/bevy/pull/6245
[6254]: https://github.com/bevyengine/bevy/pull/6254
[6284]: https://github.com/bevyengine/bevy/pull/6284
[6292]: https://github.com/bevyengine/bevy/pull/6292
[6305]: https://github.com/bevyengine/bevy/pull/6305
[6320]: https://github.com/bevyengine/bevy/pull/6320
[6388]: https://github.com/bevyengine/bevy/pull/6388
[6391]: https://github.com/bevyengine/bevy/pull/6391
[6402]: https://github.com/bevyengine/bevy/pull/6402
[6404]: https://github.com/bevyengine/bevy/pull/6404
[6412]: https://github.com/bevyengine/bevy/pull/6412
[6436]: https://github.com/bevyengine/bevy/pull/6436
[6484]: https://github.com/bevyengine/bevy/pull/6484
[6496]: https://github.com/bevyengine/bevy/pull/6496
[6503]: https://github.com/bevyengine/bevy/pull/6503
[6524]: https://github.com/bevyengine/bevy/pull/6524
[6527]: https://github.com/bevyengine/bevy/pull/6527
[6534]: https://github.com/bevyengine/bevy/pull/6534
[6539]: https://github.com/bevyengine/bevy/pull/6539
[6547]: https://github.com/bevyengine/bevy/pull/6547
[6557]: https://github.com/bevyengine/bevy/pull/6557
[6558]: https://github.com/bevyengine/bevy/pull/6558
[6560]: https://github.com/bevyengine/bevy/pull/6560
[6561]: https://github.com/bevyengine/bevy/pull/6561
[6564]: https://github.com/bevyengine/bevy/pull/6564
[6566]: https://github.com/bevyengine/bevy/pull/6566
[6571]: https://github.com/bevyengine/bevy/pull/6571
[6578]: https://github.com/bevyengine/bevy/pull/6578
[6580]: https://github.com/bevyengine/bevy/pull/6580
[6582]: https://github.com/bevyengine/bevy/pull/6582
[6587]: https://github.com/bevyengine/bevy/pull/6587
[6592]: https://github.com/bevyengine/bevy/pull/6592
[6597]: https://github.com/bevyengine/bevy/pull/6597
[6599]: https://github.com/bevyengine/bevy/pull/6599
[6602]: https://github.com/bevyengine/bevy/pull/6602
[6605]: https://github.com/bevyengine/bevy/pull/6605
[6607]: https://github.com/bevyengine/bevy/pull/6607
[6612]: https://github.com/bevyengine/bevy/pull/6612
[6618]: https://github.com/bevyengine/bevy/pull/6618
[6625]: https://github.com/bevyengine/bevy/pull/6625
[6633]: https://github.com/bevyengine/bevy/pull/6633
[6639]: https://github.com/bevyengine/bevy/pull/6639
[6643]: https://github.com/bevyengine/bevy/pull/6643
[6644]: https://github.com/bevyengine/bevy/pull/6644
[6653]: https://github.com/bevyengine/bevy/pull/6653
[6658]: https://github.com/bevyengine/bevy/pull/6658
[6663]: https://github.com/bevyengine/bevy/pull/6663
[6664]: https://github.com/bevyengine/bevy/pull/6664
[6671]: https://github.com/bevyengine/bevy/pull/6671
[6672]: https://github.com/bevyengine/bevy/pull/6672
[6673]: https://github.com/bevyengine/bevy/pull/6673
[6674]: https://github.com/bevyengine/bevy/pull/6674
[6677]: https://github.com/bevyengine/bevy/pull/6677
[6681]: https://github.com/bevyengine/bevy/pull/6681
[6683]: https://github.com/bevyengine/bevy/pull/6683
[6692]: https://github.com/bevyengine/bevy/pull/6692
[6694]: https://github.com/bevyengine/bevy/pull/6694
[6696]: https://github.com/bevyengine/bevy/pull/6696
[6698]: https://github.com/bevyengine/bevy/pull/6698
[6699]: https://github.com/bevyengine/bevy/pull/6699
[6707]: https://github.com/bevyengine/bevy/pull/6707
[6709]: https://github.com/bevyengine/bevy/pull/6709
[6720]: https://github.com/bevyengine/bevy/pull/6720
[6722]: https://github.com/bevyengine/bevy/pull/6722
[6727]: https://github.com/bevyengine/bevy/pull/6727
[6730]: https://github.com/bevyengine/bevy/pull/6730
[6732]: https://github.com/bevyengine/bevy/pull/6732
[6734]: https://github.com/bevyengine/bevy/pull/6734
[6740]: https://github.com/bevyengine/bevy/pull/6740
[6742]: https://github.com/bevyengine/bevy/pull/6742
[6743]: https://github.com/bevyengine/bevy/pull/6743
[6745]: https://github.com/bevyengine/bevy/pull/6745
[6751]: https://github.com/bevyengine/bevy/pull/6751
[6752]: https://github.com/bevyengine/bevy/pull/6752
[6755]: https://github.com/bevyengine/bevy/pull/6755
[6761]: https://github.com/bevyengine/bevy/pull/6761
[6763]: https://github.com/bevyengine/bevy/pull/6763
[6781]: https://github.com/bevyengine/bevy/pull/6781
[6785]: https://github.com/bevyengine/bevy/pull/6785
[6786]: https://github.com/bevyengine/bevy/pull/6786
[6787]: https://github.com/bevyengine/bevy/pull/6787
[6796]: https://github.com/bevyengine/bevy/pull/6796
[6797]: https://github.com/bevyengine/bevy/pull/6797
[6800]: https://github.com/bevyengine/bevy/pull/6800
[6801]: https://github.com/bevyengine/bevy/pull/6801
[6802]: https://github.com/bevyengine/bevy/pull/6802
[6807]: https://github.com/bevyengine/bevy/pull/6807
[6809]: https://github.com/bevyengine/bevy/pull/6809
[6811]: https://github.com/bevyengine/bevy/pull/6811
[6816]: https://github.com/bevyengine/bevy/pull/6816
[6817]: https://github.com/bevyengine/bevy/pull/6817
[6820]: https://github.com/bevyengine/bevy/pull/6820
[6825]: https://github.com/bevyengine/bevy/pull/6825
[6828]: https://github.com/bevyengine/bevy/pull/6828
[6829]: https://github.com/bevyengine/bevy/pull/6829
[6831]: https://github.com/bevyengine/bevy/pull/6831
[6833]: https://github.com/bevyengine/bevy/pull/6833
[6843]: https://github.com/bevyengine/bevy/pull/6843
[6847]: https://github.com/bevyengine/bevy/pull/6847
[6851]: https://github.com/bevyengine/bevy/pull/6851
[6853]: https://github.com/bevyengine/bevy/pull/6853
[6859]: https://github.com/bevyengine/bevy/pull/6859
[6865]: https://github.com/bevyengine/bevy/pull/6865
[6867]: https://github.com/bevyengine/bevy/pull/6867
[6874]: https://github.com/bevyengine/bevy/pull/6874
[6878]: https://github.com/bevyengine/bevy/pull/6878
[6881]: https://github.com/bevyengine/bevy/pull/6881
[6885]: https://github.com/bevyengine/bevy/pull/6885
[6894]: https://github.com/bevyengine/bevy/pull/6894
[6896]: https://github.com/bevyengine/bevy/pull/6896
[6899]: https://github.com/bevyengine/bevy/pull/6899
[6900]: https://github.com/bevyengine/bevy/pull/6900
[6902]: https://github.com/bevyengine/bevy/pull/6902
[6908]: https://github.com/bevyengine/bevy/pull/6908
[6912]: https://github.com/bevyengine/bevy/pull/6912
[6914]: https://github.com/bevyengine/bevy/pull/6914
[6919]: https://github.com/bevyengine/bevy/pull/6919
[6921]: https://github.com/bevyengine/bevy/pull/6921
[6922]: https://github.com/bevyengine/bevy/pull/6922
[6926]: https://github.com/bevyengine/bevy/pull/6926
[6934]: https://github.com/bevyengine/bevy/pull/6934
[6935]: https://github.com/bevyengine/bevy/pull/6935
[6936]: https://github.com/bevyengine/bevy/pull/6936
[6937]: https://github.com/bevyengine/bevy/pull/6937
[6940]: https://github.com/bevyengine/bevy/pull/6940
[6942]: https://github.com/bevyengine/bevy/pull/6942
[6946]: https://github.com/bevyengine/bevy/pull/6946
[6955]: https://github.com/bevyengine/bevy/pull/6955
[6957]: https://github.com/bevyengine/bevy/pull/6957
[6965]: https://github.com/bevyengine/bevy/pull/6965
[6970]: https://github.com/bevyengine/bevy/pull/6970
[6973]: https://github.com/bevyengine/bevy/pull/6973
[6980]: https://github.com/bevyengine/bevy/pull/6980
[6988]: https://github.com/bevyengine/bevy/pull/6988
[6995]: https://github.com/bevyengine/bevy/pull/6995
[7001]: https://github.com/bevyengine/bevy/pull/7001
[7003]: https://github.com/bevyengine/bevy/pull/7003
[7009]: https://github.com/bevyengine/bevy/pull/7009
[7010]: https://github.com/bevyengine/bevy/pull/7010
[7013]: https://github.com/bevyengine/bevy/pull/7013
[7014]: https://github.com/bevyengine/bevy/pull/7014
[7015]: https://github.com/bevyengine/bevy/pull/7015
[7016]: https://github.com/bevyengine/bevy/pull/7016
[7017]: https://github.com/bevyengine/bevy/pull/7017
[7020]: https://github.com/bevyengine/bevy/pull/7020
[7023]: https://github.com/bevyengine/bevy/pull/7023
[7024]: https://github.com/bevyengine/bevy/pull/7024
[7031]: https://github.com/bevyengine/bevy/pull/7031
[7033]: https://github.com/bevyengine/bevy/pull/7033
[7039]: https://github.com/bevyengine/bevy/pull/7039
[7040]: https://github.com/bevyengine/bevy/pull/7040
[7041]: https://github.com/bevyengine/bevy/pull/7041
[7043]: https://github.com/bevyengine/bevy/pull/7043
[7046]: https://github.com/bevyengine/bevy/pull/7046
[7048]: https://github.com/bevyengine/bevy/pull/7048
[7051]: https://github.com/bevyengine/bevy/pull/7051
[7053]: https://github.com/bevyengine/bevy/pull/7053
[7056]: https://github.com/bevyengine/bevy/pull/7056
[7060]: https://github.com/bevyengine/bevy/pull/7060
[7063]: https://github.com/bevyengine/bevy/pull/7063
[7064]: https://github.com/bevyengine/bevy/pull/7064
[7069]: https://github.com/bevyengine/bevy/pull/7069
[7076]: https://github.com/bevyengine/bevy/pull/7076
[7078]: https://github.com/bevyengine/bevy/pull/7078
[7083]: https://github.com/bevyengine/bevy/pull/7083
[7084]: https://github.com/bevyengine/bevy/pull/7084
[7087]: https://github.com/bevyengine/bevy/pull/7087
[7094]: https://github.com/bevyengine/bevy/pull/7094
[7097]: https://github.com/bevyengine/bevy/pull/7097
[7105]: https://github.com/bevyengine/bevy/pull/7105
[7113]: https://github.com/bevyengine/bevy/pull/7113
[7114]: https://github.com/bevyengine/bevy/pull/7114
[7117]: https://github.com/bevyengine/bevy/pull/7117
[7125]: https://github.com/bevyengine/bevy/pull/7125
[7127]: https://github.com/bevyengine/bevy/pull/7127
[7134]: https://github.com/bevyengine/bevy/pull/7134
[7142]: https://github.com/bevyengine/bevy/pull/7142
[7145]: https://github.com/bevyengine/bevy/pull/7145
[7146]: https://github.com/bevyengine/bevy/pull/7146
[7148]: https://github.com/bevyengine/bevy/pull/7148
[7149]: https://github.com/bevyengine/bevy/pull/7149
[7150]: https://github.com/bevyengine/bevy/pull/7150
[7151]: https://github.com/bevyengine/bevy/pull/7151
[7161]: https://github.com/bevyengine/bevy/pull/7161
[7164]: https://github.com/bevyengine/bevy/pull/7164
[7166]: https://github.com/bevyengine/bevy/pull/7166
[7174]: https://github.com/bevyengine/bevy/pull/7174
[7176]: https://github.com/bevyengine/bevy/pull/7176
[7181]: https://github.com/bevyengine/bevy/pull/7181
[7182]: https://github.com/bevyengine/bevy/pull/7182
[7186]: https://github.com/bevyengine/bevy/pull/7186
[7199]: https://github.com/bevyengine/bevy/pull/7199
[7205]: https://github.com/bevyengine/bevy/pull/7205
[7206]: https://github.com/bevyengine/bevy/pull/7206
[7222]: https://github.com/bevyengine/bevy/pull/7222
[7233]: https://github.com/bevyengine/bevy/pull/7233
[7238]: https://github.com/bevyengine/bevy/pull/7238
[7243]: https://github.com/bevyengine/bevy/pull/7243
[7245]: https://github.com/bevyengine/bevy/pull/7245
[7246]: https://github.com/bevyengine/bevy/pull/7246
[7248]: https://github.com/bevyengine/bevy/pull/7248
[7261]: https://github.com/bevyengine/bevy/pull/7261
[7262]: https://github.com/bevyengine/bevy/pull/7262
[7267]: https://github.com/bevyengine/bevy/pull/7267
[7276]: https://github.com/bevyengine/bevy/pull/7276
[7277]: https://github.com/bevyengine/bevy/pull/7277
[7279]: https://github.com/bevyengine/bevy/pull/7279
[7280]: https://github.com/bevyengine/bevy/pull/7280
[7283]: https://github.com/bevyengine/bevy/pull/7283
[7284]: https://github.com/bevyengine/bevy/pull/7284
[7290]: https://github.com/bevyengine/bevy/pull/7290
[7292]: https://github.com/bevyengine/bevy/pull/7292
[7296]: https://github.com/bevyengine/bevy/pull/7296
[7297]: https://github.com/bevyengine/bevy/pull/7297
[7298]: https://github.com/bevyengine/bevy/pull/7298
[7304]: https://github.com/bevyengine/bevy/pull/7304
[7305]: https://github.com/bevyengine/bevy/pull/7305
[7306]: https://github.com/bevyengine/bevy/pull/7306
[7311]: https://github.com/bevyengine/bevy/pull/7311
[7316]: https://github.com/bevyengine/bevy/pull/7316
[7321]: https://github.com/bevyengine/bevy/pull/7321
[7324]: https://github.com/bevyengine/bevy/pull/7324
[7325]: https://github.com/bevyengine/bevy/pull/7325
[7340]: https://github.com/bevyengine/bevy/pull/7340
[7343]: https://github.com/bevyengine/bevy/pull/7343
[7350]: https://github.com/bevyengine/bevy/pull/7350
[7351]: https://github.com/bevyengine/bevy/pull/7351
[7354]: https://github.com/bevyengine/bevy/pull/7354
[7356]: https://github.com/bevyengine/bevy/pull/7356
[7364]: https://github.com/bevyengine/bevy/pull/7364
[7370]: https://github.com/bevyengine/bevy/pull/7370
[7379]: https://github.com/bevyengine/bevy/pull/7379
[7381]: https://github.com/bevyengine/bevy/pull/7381
[7385]: https://github.com/bevyengine/bevy/pull/7385
[7387]: https://github.com/bevyengine/bevy/pull/7387
[7392]: https://github.com/bevyengine/bevy/pull/7392
[7396]: https://github.com/bevyengine/bevy/pull/7396
[7399]: https://github.com/bevyengine/bevy/pull/7399
[7401]: https://github.com/bevyengine/bevy/pull/7401
[7414]: https://github.com/bevyengine/bevy/pull/7414
[7415]: https://github.com/bevyengine/bevy/pull/7415
[7423]: https://github.com/bevyengine/bevy/pull/7423
[7425]: https://github.com/bevyengine/bevy/pull/7425
[7431]: https://github.com/bevyengine/bevy/pull/7431
[7432]: https://github.com/bevyengine/bevy/pull/7432
[7444]: https://github.com/bevyengine/bevy/pull/7444
[7445]: https://github.com/bevyengine/bevy/pull/7445
[7446]: https://github.com/bevyengine/bevy/pull/7446
[7448]: https://github.com/bevyengine/bevy/pull/7448
[7449]: https://github.com/bevyengine/bevy/pull/7449
[7452]: https://github.com/bevyengine/bevy/pull/7452
[7456]: https://github.com/bevyengine/bevy/pull/7456
[7458]: https://github.com/bevyengine/bevy/pull/7458
[7463]: https://github.com/bevyengine/bevy/pull/7463
[7466]: https://github.com/bevyengine/bevy/pull/7466
[7467]: https://github.com/bevyengine/bevy/pull/7467
[7468]: https://github.com/bevyengine/bevy/pull/7468
[7469]: https://github.com/bevyengine/bevy/pull/7469
[7471]: https://github.com/bevyengine/bevy/pull/7471
[7472]: https://github.com/bevyengine/bevy/pull/7472
[7475]: https://github.com/bevyengine/bevy/pull/7475
[7477]: https://github.com/bevyengine/bevy/pull/7477
[7478]: https://github.com/bevyengine/bevy/pull/7478
[7480]: https://github.com/bevyengine/bevy/pull/7480
[7481]: https://github.com/bevyengine/bevy/pull/7481
[7483]: https://github.com/bevyengine/bevy/pull/7483
[7489]: https://github.com/bevyengine/bevy/pull/7489
[7491]: https://github.com/bevyengine/bevy/pull/7491
[7493]: https://github.com/bevyengine/bevy/pull/7493
[7498]: https://github.com/bevyengine/bevy/pull/7498
[7503]: https://github.com/bevyengine/bevy/pull/7503
[7509]: https://github.com/bevyengine/bevy/pull/7509
[7510]: https://github.com/bevyengine/bevy/pull/7510
[7512]: https://github.com/bevyengine/bevy/pull/7512
[7514]: https://github.com/bevyengine/bevy/pull/7514
[7517]: https://github.com/bevyengine/bevy/pull/7517
[7518]: https://github.com/bevyengine/bevy/pull/7518
[7519]: https://github.com/bevyengine/bevy/pull/7519
[7520]: https://github.com/bevyengine/bevy/pull/7520
[7522]: https://github.com/bevyengine/bevy/pull/7522
[7524]: https://github.com/bevyengine/bevy/pull/7524
[7526]: https://github.com/bevyengine/bevy/pull/7526
[7527]: https://github.com/bevyengine/bevy/pull/7527
[7530]: https://github.com/bevyengine/bevy/pull/7530
[7535]: https://github.com/bevyengine/bevy/pull/7535
[7537]: https://github.com/bevyengine/bevy/pull/7537
[7539]: https://github.com/bevyengine/bevy/pull/7539
[7546]: https://github.com/bevyengine/bevy/pull/7546
[7547]: https://github.com/bevyengine/bevy/pull/7547
[7548]: https://github.com/bevyengine/bevy/pull/7548
[7559]: https://github.com/bevyengine/bevy/pull/7559
[7560]: https://github.com/bevyengine/bevy/pull/7560
[7561]: https://github.com/bevyengine/bevy/pull/7561
[7568]: https://github.com/bevyengine/bevy/pull/7568
[7574]: https://github.com/bevyengine/bevy/pull/7574
[7579]: https://github.com/bevyengine/bevy/pull/7579
[7582]: https://github.com/bevyengine/bevy/pull/7582
[7583]: https://github.com/bevyengine/bevy/pull/7583
[7586]: https://github.com/bevyengine/bevy/pull/7586
[7594]: https://github.com/bevyengine/bevy/pull/7594
[7596]: https://github.com/bevyengine/bevy/pull/7596
[7597]: https://github.com/bevyengine/bevy/pull/7597
[7598]: https://github.com/bevyengine/bevy/pull/7598
[7605]: https://github.com/bevyengine/bevy/pull/7605
[7617]: https://github.com/bevyengine/bevy/pull/7617
[7619]: https://github.com/bevyengine/bevy/pull/7619
[7623]: https://github.com/bevyengine/bevy/pull/7623
[7626]: https://github.com/bevyengine/bevy/pull/7626
[7628]: https://github.com/bevyengine/bevy/pull/7628
[7638]: https://github.com/bevyengine/bevy/pull/7638
[7639]: https://github.com/bevyengine/bevy/pull/7639
[7641]: https://github.com/bevyengine/bevy/pull/7641
[7653]: https://github.com/bevyengine/bevy/pull/7653
[7660]: https://github.com/bevyengine/bevy/pull/7660
[7664]: https://github.com/bevyengine/bevy/pull/7664
[7665]: https://github.com/bevyengine/bevy/pull/7665
[7667]: https://github.com/bevyengine/bevy/pull/7667
[7668]: https://github.com/bevyengine/bevy/pull/7668
[7671]: https://github.com/bevyengine/bevy/pull/7671
[7674]: https://github.com/bevyengine/bevy/pull/7674
[7675]: https://github.com/bevyengine/bevy/pull/7675
[7677]: https://github.com/bevyengine/bevy/pull/7677
[7681]: https://github.com/bevyengine/bevy/pull/7681
[7683]: https://github.com/bevyengine/bevy/pull/7683
[7684]: https://github.com/bevyengine/bevy/pull/7684
[7688]: https://github.com/bevyengine/bevy/pull/7688
[7696]: https://github.com/bevyengine/bevy/pull/7696
[7701]: https://github.com/bevyengine/bevy/pull/7701
[7709]: https://github.com/bevyengine/bevy/pull/7709
[7713]: https://github.com/bevyengine/bevy/pull/7713
[7715]: https://github.com/bevyengine/bevy/pull/7715
[7717]: https://github.com/bevyengine/bevy/pull/7717
[7718]: https://github.com/bevyengine/bevy/pull/7718
[7720]: https://github.com/bevyengine/bevy/pull/7720
[7723]: https://github.com/bevyengine/bevy/pull/7723
[7724]: https://github.com/bevyengine/bevy/pull/7724
[7725]: https://github.com/bevyengine/bevy/pull/7725
[7726]: https://github.com/bevyengine/bevy/pull/7726
[7727]: https://github.com/bevyengine/bevy/pull/7727
[7737]: https://github.com/bevyengine/bevy/pull/7737
[7740]: https://github.com/bevyengine/bevy/pull/7740
[7741]: https://github.com/bevyengine/bevy/pull/7741
[7744]: https://github.com/bevyengine/bevy/pull/7744
[7745]: https://github.com/bevyengine/bevy/pull/7745
[7753]: https://github.com/bevyengine/bevy/pull/7753
[7754]: https://github.com/bevyengine/bevy/pull/7754
[7755]: https://github.com/bevyengine/bevy/pull/7755
[7756]: https://github.com/bevyengine/bevy/pull/7756
[7766]: https://github.com/bevyengine/bevy/pull/7766
[7769]: https://github.com/bevyengine/bevy/pull/7769
[7782]: https://github.com/bevyengine/bevy/pull/7782
[7784]: https://github.com/bevyengine/bevy/pull/7784
[7786]: https://github.com/bevyengine/bevy/pull/7786
[7788]: https://github.com/bevyengine/bevy/pull/7788
[7790]: https://github.com/bevyengine/bevy/pull/7790
[7793]: https://github.com/bevyengine/bevy/pull/7793
[7796]: https://github.com/bevyengine/bevy/pull/7796
[7798]: https://github.com/bevyengine/bevy/pull/7798
[7800]: https://github.com/bevyengine/bevy/pull/7800
[7805]: https://github.com/bevyengine/bevy/pull/7805
[7806]: https://github.com/bevyengine/bevy/pull/7806
[7810]: https://github.com/bevyengine/bevy/pull/7810
[7815]: https://github.com/bevyengine/bevy/pull/7815
[7825]: https://github.com/bevyengine/bevy/pull/7825
[7829]: https://github.com/bevyengine/bevy/pull/7829
[7841]: https://github.com/bevyengine/bevy/pull/7841
[7846]: https://github.com/bevyengine/bevy/pull/7846
[7847]: https://github.com/bevyengine/bevy/pull/7847
[7851]: https://github.com/bevyengine/bevy/pull/7851
[7859]: https://github.com/bevyengine/bevy/pull/7859
[7860]: https://github.com/bevyengine/bevy/pull/7860
[7863]: https://github.com/bevyengine/bevy/pull/7863
[7866]: https://github.com/bevyengine/bevy/pull/7866
[7870]: https://github.com/bevyengine/bevy/pull/7870
[7875]: https://github.com/bevyengine/bevy/pull/7875
[7877]: https://github.com/bevyengine/bevy/pull/7877
[7878]: https://github.com/bevyengine/bevy/pull/7878
[7883]: https://github.com/bevyengine/bevy/pull/7883
[7884]: https://github.com/bevyengine/bevy/pull/7884
[7890]: https://github.com/bevyengine/bevy/pull/7890
[7891]: https://github.com/bevyengine/bevy/pull/7891
[7895]: https://github.com/bevyengine/bevy/pull/7895
[7897]: https://github.com/bevyengine/bevy/pull/7897
[7899]: https://github.com/bevyengine/bevy/pull/7899
[7904]: https://github.com/bevyengine/bevy/pull/7904

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
