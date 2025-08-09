---
title: Composable Specialization 
pull_requests: [17373]
---

The existing pipeline specialization APIs (`SpecializedRenderPipeline` etc.) have
been replaced with a single `Specializer` trait and `Variants` collection:

```rust
pub trait Specializer<T: Specializable>: Send + Sync + 'static {
    type Key: SpecializerKey;
    fn specialize(
        &self,
        key: Self::Key,
        descriptor: &mut T::Descriptor,
    ) -> Result<Canonical<Self::Key>, BevyError>;
}

pub struct Variants<T: Specializable, S: Specializer<T>>{ ... };
```

For more info on specialization, see the docs for `bevy_render::render_resources::Specializer`

## Mutation and Base Descriptors

The main difference between the old and new trait is that instead of
*producing* a pipeline descriptor, `Specializer`s *mutate* existing descriptors
based on a key. As such, `Variants::new` takes in a "base descriptor"
to act as the template from which the specializer creates pipeline variants.

When migrating, the "static" parts of the pipeline (that don't depend
on the key) should become part of the base descriptor, while the specializer
itself should only change the parts demanded by the key. In the full example
below, instead of creating the entire pipeline descriptor the specializer
only changes the msaa sample count and the bind group layout.

## Composing Specializers

`Specializer`s can also be *composed* with the included derive macro to combine
their effects! This is a great way to encapsulate and reuse specialization logic,
though the rest of this guide will focus on migrating "standalone" specializers.

```rust
pub struct MsaaSpecializer {...}
impl Specialize<RenderPipeline> for MsaaSpecializer {...}

pub struct MeshLayoutSpecializer {...}
impl Specialize<RenderPipeline> for MeshLayoutSpecializer {...}

#[derive(Specializer)]
#[specialize(RenderPipeline)]
pub struct MySpecializer {
    msaa: MsaaSpecializer,
    mesh_layout: MeshLayoutSpecializer,
}
```

## Misc Changes

The analogue of `SpecializedRenderPipelines`, `Variants`, is no longer a
Bevy `Resource`. Instead, the cache should be stored in a user-created `Resource`
(shown below) or even in a `Component` depending on the use case.

## Full Migration Example

Before:

```rust
#[derive(Resource)]
pub struct MyPipeline {
    layout: BindGroupLayout,
    layout_msaa: BindGroupLayout,
    vertex: Handle<Shader>,
    fragment: Handle<Shader>,
}

// before
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct MyPipelineKey {
    msaa: Msaa,
}

impl FromWorld for MyPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let asset_server = world.resource::<AssetServer>();

        let layout = render_device.create_bind_group_layout(...);
        let layout_msaa = render_device.create_bind_group_layout(...);

        let vertex = asset_server.load("vertex.wgsl");
        let fragment = asset_server.load("fragment.wgsl");
        
        Self {
            layout,
            layout_msaa,
            vertex,
            fragment,
        }
    }
}

impl SpecializedRenderPipeline for MyPipeline {
    type Key = MyPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("my_pipeline".into()),
            layout: vec![
                if key.msaa.samples() > 1 {
                    self.layout_msaa.clone()
                } else { 
                    self.layout.clone() 
                }
            ],
            vertex: VertexState {
                shader: self.vertex.clone(),
                ..default()
            },
            multisample: MultisampleState {
                count: key.msaa.samples(),
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.fragment.clone(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: ColorWrites::all(),
                })],
                ..default()
            }),
            ..default()
        },
    }
}

render_app
    .init_resource::<MyPipeline>();
    .init_resource::<SpecializedRenderPipelines<MySpecializer>>();
```

After:

```rust
#[derive(Resource)]
pub struct MyPipeline {
    // the base_descriptor and specializer each hold onto the static
    // wgpu resources (layout, shader handles), so we don't need
    // explicit fields for them here. However, real-world cases
    // may still need to expose them as fields to create bind groups
    // from, for example.
    variants: Variants<RenderPipeline, MySpecializer>,
}

pub struct MySpecializer {
    layout: BindGroupLayout,
    layout_msaa: BindGroupLayout,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, SpecializerKey)]
pub struct MyPipelineKey {
    msaa: Msaa,
}

impl FromWorld for MyPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let asset_server = world.resource::<AssetServer>();

        let layout = render_device.create_bind_group_layout(...);
        let layout_msaa = render_device.create_bind_group_layout(...);

        let vertex = asset_server.load("vertex.wgsl");
        let fragment = asset_server.load("fragment.wgsl");

        let base_descriptor = RenderPipelineDescriptor {
            label: Some("my_pipeline".into()),
            vertex: VertexState {
                shader: vertex.clone(),
                ..default()
            },
            fragment: Some(FragmentState {
                shader: fragment.clone(),
                ..default()
            }),
            ..default()
        },

        let variants = Variants::new(
            MySpecializer {
                layout: layout.clone(),
                layout_msaa: layout_msaa.clone(),
            },
            base_descriptor,
        );
        
        Self { variants }
    }
}

impl Specializer<RenderPipeline> for MySpecializer {
    type Key = MyKey;

    fn specialize(
        &self,
        key: Self::Key,
        descriptor: &mut RenderPipeline,
    ) -> Result<Canonical<Self::Key>, BevyError> {
        descriptor.multisample.count = key.msaa.samples();

        let layout = if key.msaa.samples() > 1 { 
            self.layout_msaa.clone()
        } else {
            self.layout.clone()
        };

        descriptor.set_layout(0, layout);

        Ok(key)
    }
}

render_app.init_resource::<MyPipeline>();

```
