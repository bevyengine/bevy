---
title: "`RenderStartup` Schedule"
authors: ["@IceSentry", "@andriyDev"]
pull_requests: [19841, 19885, 19886, 19897, 19898, 19901, 19912, 19926, 19999, 20002, 20024, 20124, 20147, 20184, 20194, 20195, 20208, 20209, 20210]
---

In previous versions of Bevy, render `Plugin` code had to look different than other `Plugin` code, due to how the renderer was initialized. In general, renderer resources and systems had to be added in `Plugin::finish`, separate from the typical spot: `Plugin::build`. The fact that `Plugin::finish` resulted in the correct order was a bit arbitrary / incidental.

As a step towards solving this,  **Bevy 0.17** introduces the `RenderStartup` schedule and ports many renderer resources to be initialized in `RenderStartup` with systems. This makes renderer initialization more structured and allows renderer plugin initialization to be defined "normally" in `Plugin::build`. It also allows renderer init code to benefit from the Bevy ECS scheduler, including automatic parallelization and system ordering.

In previous versions, initializing a renderer resource looked like this:

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

#[derive(Resource)]
pub struct MyRenderResource(/* ... */);

impl FromWorld for MyRenderResource {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        MyRenderResource(/* ... */)
    }
}
```

In **Bevy 0.17**, it can now be written like this:

```rust
impl Plugin for MyRenderingPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(RenderStartup, init_my_resource);
    }
}

#[derive(Resource)]
pub struct MyRenderResource(/* ... */);

fn init_my_resource(mut commands: Commands, render_device: Res<RenderDevice>) {
    commands.insert_resource(MyRenderResource(/* ... */));
}
```

We highly encourage renderer developers to port their own rendering resources to this new approach!
