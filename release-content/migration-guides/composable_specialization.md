---
title: Composable Specialization 
pull_requests: [17373]
---

The existing pipeline specialization APIs (`SpecializedRenderPipeline` etc.) have
been replaced with a single `Specialize` trait and `Specializer` collection:

```rs
pub trait Specialize<T: Specializable>: Send + Sync + 'static {
    type Key: SpecializeKey;
    fn specialize(
        &self,
        key: Self::Key,
        descriptor: &mut T::Descriptor,
    ) -> Result<Canonical<Self::Key>, BevyError>;
}

pub struct Specializer<T: Specializable, S: Specialize<T>>{ ... };
```

The main difference is the change from *producing* a pipeline descriptor to
*mutating* one based on a key. The "base descriptor" that the `Specializer`
passes to the `Specialize` implementation can either be specified manually
with `Specializer::new` or by implementing `GetBaseDescriptor`. Also, there's
a new key for specialization keys, `SpecializeKey`, that can be derived with
the included macro in most cases.

Composing multiple different specializers together with the `derive(Specialize)`
macro can be a lot more powerful (see the `Specialize` docs), but migrating
individual specializers is fairly simple:

```rs
pub struct MySpecializer {
    layout: BindGroupLayout,
    layout_msaa: BindGroupLayout,
    vertex: Handle<Shader>,
    fragment: Handle<Shader>,
}

// before
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
// after
#[derive(Clone, Copy, PartialEq, Eq, Hash, SpecializeKey)]
pub struct MyKey {
    blend_state: BlendState,
    msaa: Msaa,
}

impl FromWorld for MySpecializer {
    fn from_world(&mut World) -> Self {
        ...
    }
}

// before
impl SpecializedRenderPipeline for MySpecializer {
    type Key = MyKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("my_pipeline".into()),
            layout: vec![
                if key.msaa.samples() > 0 {
                    self.layout_msaa.clone()
                } else { 
                    self.layout.clone() 
                }
            ],
            push_constant_ranges: vec![],
            vertex: VertexState {
                shader: self.vertex.clone(),
                shader_defs: vec![],
                entry_point: "vertex".into(),
                buffers: vec![],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: MultisampleState {
                count: key.msaa.samples(),
                ..Default::default()
            },
            fragment: Some(FragmentState {
                shader: self.fragment.clone(),
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba8Unorm,
                    blend: Some(key.blend_state),
                    write_mask: ColorWrites::all(),
                })],
            }),
            zero_initialize_workgroup_memory: false,
        },
    }
}

app.init_resource::<SpecializedRenderPipelines<MySpecializer>>();

// after
impl Specialize<RenderPipeline> for MySpecializer {
    type Key = MyKey;

    fn specialize(
        &self,
        key: Self::Key,
        descriptor: &mut RenderPipeline,
    ) -> Result<Canonical<Self::Key>, BevyError> {
        descriptor.multisample.count = key.msaa.samples();
        descriptor.layout[0] = if key.msaa.samples() > 0 {
            self.layout_msaa.clone()
        } else {
            self.layout.clone()
        };
        descriptor.fragment.targets[0].as_mut().unwrap().blend_mode = key.blend_state;
        Ok(key)
    }
}

impl GetBaseDescriptor for MySpecializer {
    fn get_base_descriptor(&self) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("my_pipeline".into()),
            layout: vec![self.layout.clone()],
            push_constant_ranges: vec![],
            vertex: VertexState {
                shader: self.vertex.clone(),
                shader_defs: vec![],
                entry_point: "vertex".into(),
                buffers: vec![],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: MultiSampleState::default(),
            fragment: Some(FragmentState {
                shader: self.fragment.clone(),
                shader_defs: vec![],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: ColorWrites::all(),
                })],
            }),
            zero_initialize_workgroup_memory: false,
        },
    }
}

app.init_resource::<Specializer<RenderPipeline, MySpecializer>>();
```
