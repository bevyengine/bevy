use core::marker::PhantomData;

use crate::component::Component;
use crate::entity::{ComponentCloneCtx, SourceComponent};

/// Function type that can be used to clone a component of an entity.
pub type ComponentCloneFn = fn(&SourceComponent, &mut ComponentCloneCtx);

/// The clone behavior to use when cloning or moving a [`Component`].
#[derive(Clone, Debug, Default)]
pub enum ComponentCloneBehavior {
    /// Uses the default behavior (which is passed to [`ComponentCloneBehavior::resolve`])
    #[default]
    Default,
    /// Do not clone/move this component.
    Ignore,
    /// Uses a custom [`ComponentCloneFn`].
    Custom(ComponentCloneFn),
}

impl ComponentCloneBehavior {
    /// Set clone handler based on `Clone` trait.
    ///
    /// If set as a handler for a component that is not the same as the one used to create this handler, it will panic.
    pub fn clone<C: Component + Clone>() -> Self {
        Self::Custom(component_clone_via_clone::<C>)
    }

    /// Set clone handler based on `Reflect` trait.
    #[cfg(feature = "bevy_reflect")]
    pub fn reflect() -> Self {
        Self::Custom(component_clone_via_reflect)
    }

    /// Returns the "global default"
    pub fn global_default_fn() -> ComponentCloneFn {
        #[cfg(feature = "bevy_reflect")]
        return component_clone_via_reflect;
        #[cfg(not(feature = "bevy_reflect"))]
        return component_clone_ignore;
    }

    /// Resolves the [`ComponentCloneBehavior`] to a [`ComponentCloneFn`]. If [`ComponentCloneBehavior::Default`] is
    /// specified, the given `default` function will be used.
    pub fn resolve(&self, default: ComponentCloneFn) -> ComponentCloneFn {
        match self {
            ComponentCloneBehavior::Default => default,
            ComponentCloneBehavior::Ignore => component_clone_ignore,
            ComponentCloneBehavior::Custom(custom) => *custom,
        }
    }
}

/// Component [clone handler function](ComponentCloneFn) implemented using the [`Clone`] trait.
/// Can be [set](Component::clone_behavior) as clone handler for the specific component it is implemented for.
/// It will panic if set as handler for any other component.
///
pub fn component_clone_via_clone<C: Clone + Component>(
    source: &SourceComponent,
    ctx: &mut ComponentCloneCtx,
) {
    if let Some(component) = source.read::<C>() {
        ctx.write_target_component(component.clone());
    }
}

/// Component [clone handler function](ComponentCloneFn) implemented using reflect.
/// Can be [set](Component::clone_behavior) as clone handler for any registered component,
/// but only reflected components will be cloned.
///
/// To clone a component using this handler, the following must be true:
/// - World has [`AppTypeRegistry`](crate::reflect::AppTypeRegistry)
/// - Component has [`TypeId`](core::any::TypeId)
/// - Component is registered
/// - Component has [`ReflectFromPtr`](bevy_reflect::ReflectFromPtr) registered
/// - Component can be cloned via [`PartialReflect::reflect_clone`] _or_ has one of the following registered: [`ReflectFromReflect`](bevy_reflect::ReflectFromReflect),
///   [`ReflectDefault`](bevy_reflect::std_traits::ReflectDefault), [`ReflectFromWorld`](crate::reflect::ReflectFromWorld)
///
/// If any of the conditions is not satisfied, the component will be skipped.
///
/// See [`EntityClonerBuilder`](crate::entity::EntityClonerBuilder) for details.
///
/// [`PartialReflect::reflect_clone`]: bevy_reflect::PartialReflect::reflect_clone
#[cfg(feature = "bevy_reflect")]
pub fn component_clone_via_reflect(source: &SourceComponent, ctx: &mut ComponentCloneCtx) {
    let Some(app_registry) = ctx.type_registry().cloned() else {
        return;
    };
    let registry = app_registry.read();
    let Some(source_component_reflect) = source.read_reflect(&registry) else {
        return;
    };
    let component_info = ctx.component_info();
    // checked in read_source_component_reflect
    let type_id = component_info.type_id().unwrap();

    // Try to clone using `reflect_clone`
    if let Ok(mut component) = source_component_reflect.reflect_clone() {
        if let Some(reflect_component) =
            registry.get_type_data::<crate::reflect::ReflectComponent>(type_id)
        {
            reflect_component.map_entities(&mut *component, ctx.entity_mapper());
        }
        drop(registry);

        ctx.write_target_component_reflect(component);
        return;
    }

    // Try to clone using ReflectFromReflect
    if let Some(reflect_from_reflect) =
        registry.get_type_data::<bevy_reflect::ReflectFromReflect>(type_id)
    {
        if let Some(mut component) =
            reflect_from_reflect.from_reflect(source_component_reflect.as_partial_reflect())
        {
            if let Some(reflect_component) =
                registry.get_type_data::<crate::reflect::ReflectComponent>(type_id)
            {
                reflect_component.map_entities(&mut *component, ctx.entity_mapper());
            }
            drop(registry);

            ctx.write_target_component_reflect(component);
            return;
        }
    }
    // Else, try to clone using ReflectDefault
    if let Some(reflect_default) =
        registry.get_type_data::<bevy_reflect::std_traits::ReflectDefault>(type_id)
    {
        let mut component = reflect_default.default();
        component.apply(source_component_reflect.as_partial_reflect());
        drop(registry);
        ctx.write_target_component_reflect(component);
        return;
    }
    // Otherwise, try to clone using ReflectFromWorld
    if let Some(reflect_from_world) =
        registry.get_type_data::<crate::reflect::ReflectFromWorld>(type_id)
    {
        use crate::{entity::EntityMapper, world::World};

        let reflect_from_world = reflect_from_world.clone();
        let source_component_cloned = source_component_reflect.to_dynamic();
        let component_layout = component_info.layout();
        let target = ctx.target();
        let component_id = ctx.component_id();
        drop(registry);
        ctx.queue_deferred(move |world: &mut World, mapper: &mut dyn EntityMapper| {
            let mut component = reflect_from_world.from_world(world);
            assert_eq!(type_id, (*component).type_id());
            component.apply(source_component_cloned.as_partial_reflect());
            if let Some(reflect_component) = app_registry
                .read()
                .get_type_data::<crate::reflect::ReflectComponent>(type_id)
            {
                reflect_component.map_entities(&mut *component, mapper);
            }
            // SAFETY:
            // - component_id is from the same world as target entity
            // - component is a valid value represented by component_id
            unsafe {
                use alloc::boxed::Box;
                use bevy_ptr::OwningPtr;

                let raw_component_ptr =
                    core::ptr::NonNull::new_unchecked(Box::into_raw(component).cast::<u8>());
                world
                    .entity_mut(target)
                    .insert_by_id(component_id, OwningPtr::new(raw_component_ptr));

                if component_layout.size() > 0 {
                    // Ensure we don't attempt to deallocate zero-sized components
                    alloc::alloc::dealloc(raw_component_ptr.as_ptr(), component_layout);
                }
            }
        });
    }
}

/// Noop implementation of component clone handler function.
///
/// See [`EntityClonerBuilder`](crate::entity::EntityClonerBuilder) for details.
pub fn component_clone_ignore(_source: &SourceComponent, _ctx: &mut ComponentCloneCtx) {}

/// Wrapper for components clone specialization using autoderef.
#[doc(hidden)]
pub struct DefaultCloneBehaviorSpecialization<T>(PhantomData<T>);

impl<T> Default for DefaultCloneBehaviorSpecialization<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

/// Base trait for components clone specialization using autoderef.
#[doc(hidden)]
pub trait DefaultCloneBehaviorBase {
    fn default_clone_behavior(&self) -> ComponentCloneBehavior;
}

impl<C> DefaultCloneBehaviorBase for DefaultCloneBehaviorSpecialization<C> {
    fn default_clone_behavior(&self) -> ComponentCloneBehavior {
        ComponentCloneBehavior::Default
    }
}

/// Specialized trait for components clone specialization using autoderef.
#[doc(hidden)]
pub trait DefaultCloneBehaviorViaClone {
    fn default_clone_behavior(&self) -> ComponentCloneBehavior;
}

impl<C: Clone + Component> DefaultCloneBehaviorViaClone for &DefaultCloneBehaviorSpecialization<C> {
    fn default_clone_behavior(&self) -> ComponentCloneBehavior {
        ComponentCloneBehavior::clone::<C>()
    }
}
