---
title: Render Graph as Systems
authors: [ "@tychedelia" ]
pull_requests: [ 22144 ]
---

Bevy's `RenderGraph` architecture has been replaced with schedules. Render passes are now regular systems that run in
the `Core3d`, `Core2d`, or custom rendering schedules and executed within the render world.

The render graph was originally designed when Bevy's ECS was less mature. In order to add custom rendering
functionality, we required users to implement a trait `Node`, derive a `RenderLabel`, and use a targeted API for ordering
this rendering work relative to other tasks:

```rust
pub struct MyCustomRenderNode;

impl Node for MyCustomNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let res_a = world.resource::<Res<A>>();
        let encoder = render_context.command_encoder();

        // do some rendering things

        Ok(())
    }
}

#[derive(RenderLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub struct MyCustomRenderNodeLabel;

pub struct MyRenderPlugin;

impl Plugin for MyRenderPlugin {
    fn build(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_graph_node::<ViewNodeRunner<MyCustomNode>>(
                Core3d,
                MyCustomRenderNodeLabel
            )
            .add_render_graph_edge(
                Core3d,
                Node3d::MainPass,
                MyCustomRenderNodeLabel
            );
    }
}
```

As our APIs have evolved, `Schedule` has become capable of expressing the core render graph pattern. This change lets
rendering better leverage familiar Bevy patterns, allowing the above to be expressed as:

```rust
fn my_custom_render_system(mut ctx: RenderContext, res_a: Res<A>) {
    let encoder = ctx.command_encoder();
    // do some rendering things 
}

pub struct MyRenderPlugin;

impl Plugin for MyRenderPlugin {
    fn build(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_systems(Core3d, my_custom_render_system.after(Core3dSystems::MainPass));
    }
}
```

In the future, expressing rendering work as systems will allow us to explore performance optimizations that take
advantage of the ECS. For example, future work to support read-only schedules could help parallelizing command encoding
by enforcing that a schedule does not mutate the world. We are excited to continue to improve the experience of custom
rendering inside Bevy!
