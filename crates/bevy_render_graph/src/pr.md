# Objective

Bevy's current render graph is quite rigid; Since it's a pure-ecs system, all the inputs and outputs are fixed. This makes it both difficult to customize for third-party users and difficult to maintain internally. ViewNode and similar abstractions relieve some of the pain of querying the World, but don't succeed in making the graph more dynamic.

### Goals:
- **make custom render graphs possible, maintainable, and even easy**
	- this will have the side effect of making it much easier to maintain separate bevy render graphs for mobile, pbr, etc.
- total control for render graph authors
- automatic resource management, without compromising control
- modularity
- reusability

### Non-goals:
- data-oriented or file-based config: while these can make sense for an opinionated *existing* renderer like those AAA studios develop in-house, they can't cover all or even most use cases without becoming very clumsy. Nothing about this PR would stop someone from building this on top, however.
- beginner-friendly custom graphs: while this might even happen as a side effect of a good standard render graph library and the pass builders, users will always need to deal with `wgpu` if they need custom nodes.
- "add custom pass from anywhere" API: the graph author must always have total control, so even though this style of API is useful for things such as post-processing, it should be in the form of a plugin + plain graph config functions that the graph author opts in to.

## Solution
> Credit to @JMS55 for guiding many of the design decisions for the new graph! I had more than a few iterations on the concept up to this point, most of them bad.

The main differences between the old and new graph are as follows: 
- while the old graph is static and created ahead of time, the new one is fully dynamic and rebuilt every frame.
- The graph has no notion of a "view entity" and manages the rendering loop itself (**THIS IS NOT FINAL AND IS UP FOR DEBATE, SEE THE SECTION BELOW**)
- while the old graph is configured indirectly from many different areas of code, the new system uses a single builder function, easily readable from a single file:

```rust
fn bevy_default_render_graph<'g>(graph: &mut RenderGraphBuilder<'g>) {
	let ui = default_ui_graph(graph);
	for view in graph.world_query::<Entity, With<ExtractedView>>() {
		bevy_default_render_graph_per_view(graph, view, ui);
	}
}

fn bevy_default_render_graph_per_view<'g>(graph: &mut RenderGraphBuilder<'g>, view: Entity, ui: RenderHandle<'g, Texture>) {
	//all graph operations live here, as plain functions
	let view_target = graph.new_resource(TextureDescriptor { ... });
	main_pass(view, view_target, ...);
	post_processing_pass(view, view_target, ...);
}
```

### RenderHandle: graph resources as IDs
The render graph defers much of its resource creation until after graph configuration is done, so it provides an opaque handle to resources that may not yet exist in the form of `RenderHandle`. These are lifetimed to the current graph execution, so it's impossible to use them outside of their original context. See the migration guide for more detail about resource creation and the utilities the graph provides.

### Render graph nodes
In the current render graph, a node is a simple unit of rendering work with defined inputs and outputs. The same applies here, except we also track which resources a node reads from and which it writes to. 

In order to provide the simplest API, graph.add_node takes a plain closure (and a `RenderDependencies`, discussed in the migration guide) with some normal rendering necessities, as well as something called `NodeContext`. `NodeContext::get()` allows dereferencing a handle into a normal resource, and then you can do whatever you want!
```rust
let texture = graph.new_resource(TextureDescriptor {...});
graph.add_node(deps![&mut texture], |ctx, _, queue, _| {
	queue.write_texture(ctx.get(texture).as_image_copy(), ...);
})
```
From there, rendering features can mostly be reduced to plain functions that take an `&mut RenderGraphBuilder` as well as handles to whatever inputs they need!

### Debate: single entry-point/view-less vs. dynamic view queuing
| Single Entry Point | Multiple entry points |
|--------------------|-----------------------|
| single builder callback | one builder callback per-view |
| simpler backend | simpler end-user experience | 
| how to effectively customize? | how to get outputs of other graphs? |

Where a single entry point has no concept of "view entities" and manages the rendering loop itself, associating each view (and things that aren't views, like UI) with an entity allows better modularity and separating concerns. The main issue is how to order these graphs and pass data between them (in the case of UI especially). The simplest possible API would look like `graph.depends_on(entity)` where `entity` is any entity with a graph attached, for example `graph.world_query_filtered::<Entity, With<UiView>>().single()`.  However, this would likely require `unsafe` code and untyped pointers to manage.

In the interest of simplicity for this initial PR, Jasmine convinced me to stay with a single entry-point system, though I did want to show what the alternative would be if we put the extra effort in. If the maintainers/community decide it's worth it to have those extra features immediately, I don't mind delaying the PR to add those features.

NOTE: this would not allow configuring a single graph from multiple places. Merely configuring multiple separate graphs, each operating on their own "view." (need a better word for this. not all views are cameras or shadows)

### Current Limitations
- slightly bad backend storage currently (just a bunch of hashmaps). In a future PR I'd like to follow-up with better data structures/id allocation since IDs are dense anyway.
- no dynamic queuing of views 
- no retained resources (`graph.last_frame(texture)`, this requires more design work),
- no automatic pass reordering
- no compute pass merging
- no automatic pass cullling
- no texture/buffer reuse between frames

### To-do for this PR
 - [ ] Unit tests
 - [ ] Documentation
 - [ ] Render pass builders
 - [ ] Extract into new crate? (maintainer/SME decision)

## Testing
This is intended to be merged as an experimental feature. The basic API should be in its final form, and essentially ready for production. Unit tests are in progress, as is better documentation, though large-scale testing will essentially have to happen as we proceed with the renderer refactor.

## Migration Guide
Since actual *migration* will happen in the form of the **big refactor™️**, this will consist of a usage/style guide and some of my ideas on what a "graph standard library" might look like. This might seem out of place for a PR, but for such a big new system I figure it would help maintainers figure out what they're looking at.

Lifetimes and types added for comprehension. These are generally inferred :)

### How do I make a resource?
There are a few ways to create graph resources:
```rust
fn my_graph<'g>(graph: &mut RenderGraphBuilder, owned_buffer: Buffer) {

	//you can create resources from descriptors, or anything that implements IntoRenderResource
	let new_texture: RenderHandle<'g, Texture> = graph.new_resource(TextureDescriptor {...})
	let my_buffer_1: &'g GpuArrayBuffer<...> = graph.world_resource::<Foo>().my_array_buffer();
	let new_buffer: RenderHandle<'g, Buffer> = graph.new_resource(my_buffer_1);

	//...or borrow them from the world
	let my_buffer_2: &'g Buffer = graph.world_resource::<Foo>().my_buffer();
	let imported_buffer: RenderHandle<'g, Buffer> = graph.import_resource(my_buffer);
	
	//...or move them into the world (lesser used)
	let owned_buffer: RenderHandle<'g, Buffer> = graph.into_resource(owned_buffer);
}
```
Note: when borrowing/taking an already-made resource from the World, users can optionally supply a descriptor that matches that resource. This is encouraged as a general practice, so functions in the graph can inspect the properties of those resources.

### What is RenderDependencies, and why doesn't my node work?
`RenderDependencies` marks what resources your node reads from and writes to, in order to properly track ordering dependencies between nodes. This must be manually specified, since the only way to infer it would be to intercept all rendering calls (think Unity's SRP render graph) which I felt would be both too complicated and worse to use. If you try to get a resource from the node context which isn't declared in the dependencies, the graph will panic. It can't detect if you write to a resource declared as read-only or vice-versa, so that's up to you.

Note: the `deps![]` macro works like `vec![]` and infers usage based on using a mutable or immutable reference (see trait `IntoRenderDependencies`). This is the preferred way to create a `RenderDependencies`.

```rust
let my_color_attachment = graph.new_resource(...);
let my_bind_group = graph.new_resource(...);
let my_pipeline = graph.new_resource(...);

//wrong
graph.add_node(deps![&my_bind_group, &my_pipeline], |ctx, _, _, cmds| {
	let bind_group = ctx.get(my_bind_group);
	let my_pipeline = ctx.get(my_bind_group);
	let color_attachment = ctx.get(my_color_attachment); //panic!
});

//right ("write", hehe)
graph.add_node(deps![&my_bind_group, &my_pipeline, &mut my_color_attachment], |ctx, _, _, cmds| {
	let bind_group = ctx.get(my_bind_group);
	let my_pipeline = ctx.get(my_bind_group);
	let color_attachment = ctx.get(my_color_attachment); //panic!
});
```
### .add_usages()
Oh no! I have a function that creates a texture/buffer and gives it back to the user, but I don't know what usages to assign! Have no fear, citizen, for the render graph tracks this for you! (sort of). For resources created by descriptor, `.add_usages()` will add the specified usage flags to the descriptor, since the resource hasn't actually been created yet. Otherwise, if the resource is imported and has an associated descriptor, the graph will panic if the needed usage isn't present. If it doesn't, the graph defers error detection to wgpu.

### .is_fresh()
Use `graph.is_fresh(resource_handle)` to check if a resource has been written to yet in the current frame or not. This is most useful when determining if a render pass should clear the color attachment or not.

### .descriptor() and .layout()
Use `graph.descriptor()` and `graph.layout()` to get the descriptor of a handle or the layout of a bind group handle respectively. This is meant to reduce parameter bloat when effects need to produce textures of the same size as their input, for example. Or, for the bind group case, when creating a pipeline given only a handle to a bind group.

### In-depth example: a full-screen render pass
```rust
pub fn full_screen_pass<'g, U: ShaderType>(graph: &mut RenderGraphBuilder<'g>, target: RenderHandle<'g, TextureView>, shader: Handle<Shader>, uniforms: U, clear_color: wgpu::Color) -> {
	let layout = graph.new_resource(&BindGroupLayoutEntries::single(
		ShaderStages::Fragment,
		uniform_buffer::<U>(false)
	));

	//will probably want to design a utility for proper use of dynamic offsets for bindings of same type
	let mut uniform_buffer = DynamicUniformBuffer::<U>::default();
	uniforms.push(uniforms);
	//note: implementations of IntoRenderResource are not yet in place for the render_resource abstractions
	let uniform_buffer_handle = graph.new_resource(&mut uniforms);
	graph.add_usages(uniform_buffer_handle, BufferUsages::BUFFER_BINDING);

	//no BindGroupEntries-style utility for this, though I'd like to have one eventually
	let bind_group = graph.new_resource(RenderGraphBindGroup {
		label: Some("full_screen_draw_bind_group");
		layout,
		entries: vec![RenderGraphBufferBinding { buffer: uniform_buffer_handle, offset: 0, size: None }]
	});

	let pipeline = graph.new_resource(RenderGraphRenderPipelineDescriptor {
		//normal fullscreen pipeline, just use a vec of layout handles instead of a vec of raw layouts
	});

	let should_clear: bool = graph.is_fresh(target);
	
	graph.add_node(deps![&pipeline, &bind_group, &mut target], |ctx, _, _, cmds| {
		let mut render_pass = cmds.begin_render_pass(&RenderPassDescriptor {
			color_attachments: vec![Some(RenderPassColorAttachment {
				view: ctx.get(target),
				resolve_target: None,
				ops: Operations {
					load: if (should_clear) { LoadOp::Clear(clear_color) } else { LoadOp::Load }
					store: StoreOp::Store
				}
			})],
			..default::Default()
		});
		render_pass.set_bind_group(0, ctx.get(bind_group), &[]);
		render_pass.set_render_pipeline(ctx.get(pipeline));
		render_pass.draw(0..3, 0..1);
	});
}
```

### The "graph standard library"
I think the structure of the new `bevy_render_graph` crate could be as follows: 
(actually moving to a new crate pending render refactor discussion)

- ::core
	- internal render graph implementation, exports core graph and descriptor types and implementations of `IntoRenderResource`
- ::std
	- re-exported at crate root, intended to be used as `bevy::render_graph::*`
	- re-exports `super::core::*`
	- miscellaneous utilities at the root: `hi_z(...)`, `DoubleBuffer<'g, R>`, Jasmine's render pass builders
	- scoped utilities and passes in sub-modules, intended to be used through qualified names: `fx::gtao::simple(...)`, `blit::one(...)`, `blit::many(...)`, `gbuffer::extract_normals(...)`,

A goal of the graph standard library is to avoid requiring a plugin for as many of these utilities as possible, and if one is required it should be very clearly documented.

Once this PR is merged and the render refactor working group starts up I'll write up a larger list of the utilities I want to see in the graph lib.

