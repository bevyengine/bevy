---
title: Render Graph as Systems
pull_requests: [ 22144 ]
---

The `RenderGraph` API has been removed. Render passes are now systems that run in `Core3d` or `Core2d` schedules.

Before:

```rust,ignore
impl ViewNode for MyNode {
    type ViewQuery = (&'static ExtractedCamera, &'static ViewTarget);

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (camera, target): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        // ...
    }
}

render_app
    .add_render_graph_node::<ViewNodeRunner<MyNode>>(Core3d, MyLabel)
    .add_render_graph_edges(Core3d, (Node3d::Foo, MyLabel, Node3d::Bar));
```

After:

```rust,ignore
pub fn my_render_pass(
    world: &World,
    view: ViewQuery<(&ExtractedCamera, &ViewTarget)>,
    mut ctx: RenderContext,
) {
    let (camera, target) = view.into_inner();
    // ...
}

render_app.add_systems(
    Core3d,
    my_render_pass
        .after(foo_pass)
        .before(bar_pass)
        .in_set(Core3dSystems::MainPass),
);
```

The `ViewNode` trait is replaced by a regular system using the `ViewQuery` parameter. `RenderContext` is now a system
parameter instead of being passed as `&mut`. Use `.before()` / `.after()` with the actual system functions (e.g.,
`main_opaque_pass_3d`) rather than `Node3d` labels.

System sets `Core3dSystems::Prepass`, `MainPass`, and `PostProcess` are available for coarse ordering. The `RenderGraph`
schedule remains as the top-level schedule for non-camera rendering.
