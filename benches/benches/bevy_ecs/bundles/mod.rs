use bevy_ecs::{
    bundle::{BoundedBundleKey, Bundle, ComponentsFromBundle},
    component::{ComponentId, ComponentsRegistrator, RequiredComponents, StorageType},
    ptr::OwningPtr,
};
use criterion::criterion_group;

mod spawn_many;
mod spawn_many_zst;
mod spawn_one_zst;

criterion_group!(
    benches,
    spawn_one_zst::spawn_one_zst,
    spawn_many_zst::spawn_many_zst,
    spawn_many::spawn_many,
);

struct MakeDynamic<B>(B);

// SAFETY:
// - setting is_static and is_bounded to false is always safe and makes cache_key irrelevant
// - everything else is delegated to B, which is a valid Bundle
unsafe impl<B: Bundle> Bundle for MakeDynamic<B> {
    fn is_static() -> bool {
        false
    }

    fn is_bounded() -> bool {
        false
    }

    fn cache_key(&self) -> BoundedBundleKey {
        BoundedBundleKey::empty()
    }

    fn component_ids(
        &self,
        components: &mut ComponentsRegistrator,
        ids: &mut impl FnMut(ComponentId),
    ) {
        self.0.component_ids(components, ids);
    }

    fn register_required_components(
        &self,
        components: &mut ComponentsRegistrator,
        required_components: &mut RequiredComponents,
    ) {
        self.0
            .register_required_components(components, required_components);
    }
}

impl<B: ComponentsFromBundle> ComponentsFromBundle for MakeDynamic<B> {
    type Effect = <B as ComponentsFromBundle>::Effect;

    fn get_components(self, func: &mut impl FnMut(StorageType, OwningPtr<'_>)) -> Self::Effect {
        self.0.get_components(func)
    }
}
