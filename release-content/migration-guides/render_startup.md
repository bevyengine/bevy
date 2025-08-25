---
title: Many render resources now initialized in `RenderStartup`
pull_requests: [19841, 19885, 19886, 19897, 19898, 19901, 19912, 19926, 19999, 20002, 20024, 20124, 20147, 20184, 20194, 20195, 20208, 20209, 20210]
---

Many render resources are **no longer present** during `Plugin::finish`. Instead they are
initialized during `RenderStartup` (which occurs once the app starts running). If you only access
the resource during the `Render` schedule, then there should be no change. However, if you need one
of these render resources to initialize your own resource, you will need to convert your resource
initialization into a system.

The following are the (public) resources that are now initialized in `RenderStartup`.

- `CasPipeline`
- `FxaaPipeline`
- `SmaaPipelines`
- `TaaPipeline`
- `ShadowSamplers`
- `GlobalClusterableObjectMeta`
- `FallbackBindlessResources`
- `AutoExposurePipeline`
- `MotionBlurPipeline`
- `SkyboxPrepassPipeline`
- `BlitPipeline`
- `DepthOfFieldGlobalBindGroupLayout`
- `DepthPyramidDummyTexture`
- `OitBuffers`
- `PostProcessingPipeline`
- `TonemappingPipeline`
- `BoxShadowPipeline`
- `GradientPipeline`
- `UiPipeline`
- `UiMaterialPipeline<M>`
- `UiTextureSlicePipeline`
- `ScreenshotToScreenPipeline`
- `VolumetricFogPipeline`
- `DeferredLightingLayout`
- `CopyDeferredLightingIdPipeline`
- `RenderLightmaps`
- `PrepassPipeline`
- `PrepassViewBindGroup`
- `Wireframe3dPipeline`
- `ScreenSpaceReflectionsPipeline`
- `MaterialPipeline`
- `MeshletPipelines`
- `MeshletMeshManager`
- `ResourceManager`
- `Wireframe2dPipeline`
- `Material2dPipeline`
- `SpritePipeline`
- `Mesh2dPipeline`
- `BatchedInstanceBuffer<Mesh2dUniform>`

The vast majority of cases for initializing render resources look like so (in Bevy 0.16):

```rust
impl Plugin for MyRenderingPlugin {
    fn build(&self, app: &mut App) {
        // Do nothing??
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<MyRenderResource>();
        render_app.add_systems(Render, my_render_system);
    }
}

pub struct MyRenderResource {
    ...
}

impl FromWorld for MyRenderResource {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let render_adapter = world.resource::<RenderAdapter>();
        let asset_server = world.resource::<AssetServer>();

        MyRenderResource {
            ...
        }
    }
}
```

The two main things to focus on are:

1. The resource implements the `FromWorld` trait which collects all its dependent resources (most
    commonly, `RenderDevice`), and then creates an instance of the resource.
2. The plugin adds its systems and resources in `Plugin::finish`.

First, we need to rewrite our `FromWorld` implementation as a system. This generally means
converting calls to `World::resource` into system params, and then using `Commands` to insert the
resource. In the above case, that would look like:

```rust
// Just a regular old system!!
fn init_my_resource(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_adapter: Res<RenderAdapter>,
    asset_server: Res<AssetServer>,
) {
    commands.insert_resource(MyRenderResource {
        ...
    });
}
```

Each case will be slightly different. Two notes to be wary of:

1. Functions that accept `&RenderDevice` for example may no longer compile after switching to
    `Res<RenderDevice>`. This can be resolved by passing `&render_device` instead of
    `render_device`.
2. If you are using `load_embedded_asset(world, "my_asset.png")`, you may need to first add
    `asset_server` as a system param, then change this to
    `load_embedded_asset(asset_server.as_ref(), "my_asset.png")`.

Now that we have our initialization system, we just need to add the system to `RenderStartup`:

```rust
impl Plugin for MyRenderingPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_systems(RenderStartup, init_my_resource)
            .add_systems(Render, my_render_system);
    }

    // No more finish!!
}
```

In addition, if your resource requires one of the affected systems above, you will need to use
system ordering to ensure your resource initializes after the other system. For example, if your
system uses `Res<UiPipeline>`, you will need to add an ordering like:

```rust
render_app.add_systems(RenderStartup, init_my_resource.after(init_ui_pipeline));
```
