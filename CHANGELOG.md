<!-- MD024 - We want repeated headings in a changelog file -->
<!-- markdownlint-disable-file MD024 -->

# Changelog

While we try to keep the `Unreleased` changes updated, it is often behind and does not include
all merged pull requests. To see a list of all changes since the latest release, you may compare
current changes on git with [previous release tags][git_tag_comparison].

[git_tag_comparison]: https://github.com/bevyengine/bevy/compare/v0.6.0...main

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
