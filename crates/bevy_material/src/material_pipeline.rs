use std::marker::PhantomData;

use bevy_app::Plugin;
use bevy_asset::AssetPath;
use bevy_ecs::system::{ReadOnlySystemParam, SystemParamItem};
use bevy_reflect::TypePath;
use bevy_render::{
    render_resource::{CachedRenderPipelineId, RenderPipelineDescriptor},
    renderer::RenderDevice,
    Render,
};
use bevy_utils::HashMap;
use variadics_please::all_tuples;

use crate::material::Material;

pub trait MaterialPipeline: TypePath + Sized + 'static {
    type Properties: Send + Sync;
    type Pipelines<M: Material<Self>>: Pipelines;

    fn material_plugin<M: Material<Self>>() -> impl Plugin;
}

pub trait Pipelines {
    type Cached: Send + Sync;

    //TODO
}

// The goal here is something like:
// ```rust
// impl Material<Mesh3d> for StandardMaterial {
//     fn metadata(&self) -> Mesh3dMetadata {
//         Mesh3dMetadata {
//             ...
//         }
//     }
//
//     fn pipelines() -> Mesh3dPipelines {
//         Mesh3dPipelines {
//             prepass: MaterialRenderPipeline::new(
//                 "my_vertex_path".into(),
//                 "my_fragment_path".into(),
//             ).specialize(my_specialization_fn)
//             deferred: ...
//             main_pass: ...
//         }
//     }
// }
// ```

pub struct MaterialRenderPipeline<S: Specialize<RenderPipelineDescriptor>> {
    vertex_path: Option<AssetPath<'static>>,
    fragment_path: Option<AssetPath<'static>>,
    _data: PhantomData<S>, // wire up the specializer somewhere in here
}

pub trait Specialize<T>: Send + Sync + 'static {
    type Key: Clone + Hash + Eq;

    fn specialize(&self, key: Self::Key, item: &mut T);

    fn chain<S: Specialize>(self, next: S) -> impl Specialize<T> {
        (self, next)
    }
}

impl<K: Clone + Hash + Eq, T, F: Fn(&self, K, &mut T)> Specialize<T> for F {
    type Key = K;

    fn specialize(&self, key: Self::Key, item: &mut T) {
        (self)(key, item)
    }
}

macro_rules! impl_specialize {
    ($(#[$meta:meta])* $(($S: ident, $s: ident, $k: ident)),*) => {
        $(#[$meta])*
         impl<T, $($S: Specialize<T>),*> Specialize<T> for ($($T,)*) {
            type Key = ($($T,)*);

            fn specialize(&self, key: Self::Key, item: &mut T) {
                let ($($s,)*) = self;
                let ($($k,)*) = key;
                $($s.specialize($k, item);)*
            }
        }
    }
}

all_tuples!(
    #[doc(fake_variadic)]
    impl_specialize,
    0,
    15,
    S,
    s,
    k
);

pub struct RenderPipelineSpecializer<S: Specialize<RenderPipelineDescriptor>> {
    specializer: S,
    pipelines: HashMap<S::Key, CachedRenderPipelineId>,
    base: RenderPipelineDescriptor,
}

impl<S: Specialize<RenderPipelineDescriptor>> RenderPipelineSpecializer<S> {
    fn specialize(&self, render_device: &RenderDevice, key: S::Key) -> CachedRenderPipelineId {
        todo!()
    }
}
