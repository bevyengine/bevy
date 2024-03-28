# Explanation of the RenderFeature stuff

## Feature 
- a weird version of `Plugin` specific to rendering
- The type parameter G allows for configuring the plugin based on its trait impls
  - in my other write-up I made the case for this 
- Defines a multi-input multi-output `Signature: RenderSignature<true>`,
  - like a function signature, it defines the inputs and outputs of a render feature
  - the `<true>` in `RenderSignature<true>` indicates that it's multi-output (described below in the RenderIO section)
- defines a compatibility key type and method `get_compatibility` to evaluate if a render feature is compatible with the current platform
- defines a method `build` for registering normal app things like Resources and plain systems
  - note: this is up for debate, if dynamically rebuilding a render- (-procedure? -graph?) is important, then features need to keep track of the things they insert to the App so they can be removed

TLDR: The purpose of a Feature is to wrap a set of SubFeatures, define the control flow between them, add additional stuff to App, allow configuration, and handle dependencies.

### Separating dependencies and control flow
In order to allow proper dependency injection and modularity, we need to separate data processing from dependency specification, in this case into two separate methods:

#### `dependencies`: 
- specifies default dependencies for a Feature, able to be overriden when adding the Feature to the RenderApp
- provides a builder for getting the output handles of another feature known at compile time/by type id
- provides methods `map` and `map_many` for adapting dependencies into usable data (automatically inserts small adapter sub-features)
  - idea for future: weld these into each system as much as possible to reduce archetype bloat
- must return handles for the input set the `Feature` expects

Note: one utility is the Handles! macro, which when writing a handles tuple replaces any `_` with a `RenderHandle::hole()`, an unfilled dependency which causes a panic unless it's overriden by the end user

#### `build_feature`: 
- where sub-features are registered and their control-flow specified
- the feature inputs are mapped through some sub-features, to the outputs
  - since the RenderHandles are the mechanism for this, we don't need any awkward configuration

### small `Feature` example: 

```rs
struct MyFeature;

struct Thingamabob { a: CachedTexture, b: CachedTexture }

impl <G: RenderSubGraph> Feature<G> for Blit {
  type Signature = Sig![(Foo, Bar, Baz) => (Thingamabob,)];

  fn dependencies<'s, 'b: 's>(
    &'s self,
    _compatibility: Self::CompatibilityKey,
    mut builder: FeatureDependencyBuilder<'b, G, Self>,
  ) -> RenderHandles<'b, true, FeatureInput<G, Self>> {
    let (foo,) = builder.with_dep::<FooFeature>();
    let (bar,) = builder.with_dep::<BarFeature>();
    Handles!(foo, bar, _) // user must specify where Baz comes from
  }

  fn build_feature<'s, 'b: 's>(
    &'s self,
    compatibility: Self::CompatibilityKey,
    builder: &'b mut FeatureBuilder<'b, G, Self>,
    (foo, bar, baz): RenderHandles<'b, true, FeatureInput<G, Self>>,
  ) -> RenderHandles<'b, true, FeatureOutput<G, Self>> {
    let first_texture = builder.add_sub_feature((foo, bar), |_, (foo, bar)| { todo!() }); // macro possibility for making this less verbose
    let second_texture = builder.add_sub_feature((baz,), |_, (baz,)| );
    let thingamabob = builder.add_sub_feature((first_texture, second_texture), |_, (a, b)| { Thingamabob { a, b } });
    (thingamabob,)
  }
}
```

## SubFeature

- basically, an abstraction over iterating extracted `Views`, very common in rendering
- they take in a set of inputs (in the backend: dynamic components on the view) and output a single new dynamic component to be added to that View
- Their `Signature`s must impl `RenderSignature<false>`, since they don't support multiple output (should they?)
- Closures/functions of the signature `Fn(Entity, (Input..), (SystemParams..)) -> Output` implement SubFeature

### `SubFeature` example:

```rs 
pub fn new_texture(
  entity: Entity,
  (format, size): (TextureFormat, Extents3d),
  (texture_cache, render_device): (ResMut<TextureCache>,)
) -> CachedTexture {
  texture_cache.get(&render_device, TextureDescriptor {
    label: None,
    size,
    mip_level_count: 1,
    sample_count: 1,
    dimension: TextureDimension::D2
    format,
    usage: TextureUsages::TEXTURE_BINDING
  })
}
```

## RenderHandle 
- A typed handle to a dynamic component id (the output of a sub feature), with some extra metadata. The lifetime on it limits its use to the scope of the relevant builder. It also has a `hole()` variant representing an unfulfilled dependency, which must be later overriden by the user or a `panic!` will occur.

## RenderSignature

- has two associated types: `In: RenderIO<true>` and `Out: RenderIO<MULTI_OUTPUT>`. Always multiple-input, multiple-output if the const parameter `MULTI_OUTPUT` is `true`

## RenderIO<const MULT: bool>

- `RenderIO<false>`: a marker trait implemented for all `Send + Sync + 'static` types and tuples. The `Handles` associated with any `RenderIO<false>` is always considered a single value, so turns into a `RenderHandle<Self>`
- `RenderIO<true>`: a marker trait for all tuples of `RenderIO<false>`, used for multi-input and multi-output features and sub-features since its `Handles` type is tupled as well:
  - note: derive macro implementing multi input-output for structs of handles?

```rs 
<(u8, u8, u8) as RenderIO<true>>::Handles = (RenderHandle<u8>, RenderHandle<u8>, RenderHandle<u8>);
```

# How will Features be added to the World? 

I don't know. They need to be stored per-graph/procedure/whatever, where graph/procedure/whatever is the name for the render sub graph itself and all its supporting systems. IMO this should be done with a single large builder and submitted a single time, so that a procedure could be reconfigured and resubmitted at will if settings change. But, I don't know if that's even possible or the best way to achieve dynamism.

# How to disambiguate resources between same-named sub-features? 

Again an open question, but I think a wrapper resource with a `HashMap<SubFeatureId, MyResource>` or something couldn't be the worst idea.
