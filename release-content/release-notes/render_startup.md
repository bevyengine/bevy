---
title: `RenderStartup` and making the renderer my ECS-y
authors: ["@IceSentry", "@andriyDev"]
pull_requests: [19841, 19926, 19885, 19886, 19897, 19898, 19901]
---

Previous rendering code looked quite different from other Bevy code. In general, resources were
initialized with the `FromWorld` trait (where most Bevy code only uses the `Default` trait for most
resources) and systems/resources were added in `Plugin::finish` (where nearly all Bevy code does not
use `Plugin::finish` at all). This difference with Bevy code can make it harder for new developers
to learn rendering, and can result in "cargo cult" copying of rendering code (e.g., "is it important
to be using `FromWorld` here? Better to be safe and just do what the rendering code is doing!").

As a step towards making the renderer more accessible (and maintainable), we have introduced the
`RenderStartup` schedule and ported many rendering resources to be initialized in `RenderStartup`
with systems! This has several benefits:

1. Creating resources in systems makes it clearer that rendering resources **are just regular
    resources**. Hopefully, this better communicates that how you initialize these resources is
    totally up to you!
2. We can now use the system ordering API to ensure that resources are initialized in the correct
    order. For example, we can do `init_material_pipeline.after(init_mesh_pipeline)` if we need the
    mesh pipeline to initialize the material pipeline.
3. These initialization systems clearly describe what resources they require through their argument
    list. If a system has an argument of `deferred_lighting_layout: Res<DeferredLightingLayout>`, it
    clearly documents that this system needs to be run **after** we initialize the
    `DeferredLightingLayout`.

We want developers to become more familiar and comfortable with Bevy's rendering stack, and hope
that bringing the renderer closer to regular ECS code will encourage that. Code that previously looked
like this (in Bevy 0.16):

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

Can now be written like:

```rust
impl Plugin for MyRenderingPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(RenderStartup, init_my_resource);
    }

    // No more finish!!
}

pub struct MyRenderResource {
    ...
}

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

We highly encourage users to port their own rendering resources to this new system approach (and for
resources whose initialization depends on a Bevy core resource, it may be required). In fact, we
encourage users to abandon `Plugin::finish` entirely and move all their system and resource
initializations for rendering into `Plugin::build` instead.

As stated before, we've ported many resources to be initialized in `RenderStartup`. See the
migration guide for a full list of affected resources.
