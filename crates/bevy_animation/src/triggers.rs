use std::any::Any;

use bevy_ecs::prelude::*;
use bevy_reflect::{
    prelude::*, utility::NonGenericTypeInfoCell, ApplyError, DynamicTupleStruct, FromType,
    GetTypeRegistration, ReflectFromPtr, ReflectKind, ReflectMut, ReflectOwned, ReflectRef,
    TupleStructFieldIter, TupleStructInfo, TypeInfo, TypeRegistration, Typed, UnnamedField,
};

pub(crate) fn trigger_animation_event(
    event: Box<dyn PartialReflect>,
    entity: Entity,
) -> impl Command {
    move |world: &mut World| {
        let (from_reflect, animation_event) = {
            let registry = world
                .get_resource::<AppTypeRegistry>()
                .expect("Missing resource `AppTypeRegistry`");
            let lock = registry.read();
            let type_info = event.get_represented_type_info().unwrap(); // FIXME: when would this fail?
            let registration = lock
                .get_with_type_path(type_info.type_path())
                .unwrap_or_else(|| {
                    panic!(
                        "Missing type registration for type: `{}`",
                        type_info.type_path()
                    )
                });
            (
                registration
                    .data::<ReflectFromReflect>()
                    .cloned()
                    .unwrap_or_else(|| {
                        panic!(
                            "Type `{}` is not registered with data: `ReflectFromReflect`",
                            type_info.type_path()
                        )
                    }),
                registration
                    .data::<ReflectAnimationEvent>()
                    .cloned()
                    .unwrap_or_else(|| {
                        panic!(
                            "Type `{}` is not registered with data: `ReflectAnimationEvent`",
                            type_info.type_path()
                        )
                    }),
            )
        };
        let event = from_reflect.from_reflect(event.as_ref()).unwrap(); // FIXME: when would this fail?
        animation_event.trigger(event.as_ref(), entity, world);
    }
}

pub trait AnimationEvent: Event + Reflect + Clone {}

#[derive(Clone)]
pub struct ReflectAnimationEvent {
    trigger: fn(&dyn Reflect, Entity, &mut World),
}

impl ReflectAnimationEvent {
    /// # Panics
    ///
    /// Panics if the underlying type of `event` does not match the type this `ReflectAnimationEvent` was constructed for.
    pub(crate) fn trigger(&self, event: &dyn Reflect, entity: Entity, world: &mut World) {
        (self.trigger)(event, entity, world)
    }
}

impl<T: AnimationEvent> FromType<T> for ReflectAnimationEvent {
    fn from_type() -> Self {
        Self {
            trigger: |value, entity, world| {
                let event = value.downcast_ref::<T>().unwrap().clone();
                world.entity_mut(entity).trigger(event);
            },
        }
    }
}

#[derive(TypePath, Debug)]
pub(crate) struct AnimationTriggerData(pub(crate) Box<dyn PartialReflect>);

impl AnimationTriggerData {
    pub(crate) fn new(event: impl Event + PartialReflect) -> Self {
        Self(Box::new(event))
    }
}

impl Clone for AnimationTriggerData {
    fn clone(&self) -> Self {
        Self(self.0.clone_value())
    }
}

impl GetTypeRegistration for AnimationTriggerData {
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<Self>();
        registration.insert::<ReflectFromPtr>(FromType::<Self>::from_type());
        registration
    }
}

impl Typed for AnimationTriggerData {
    fn type_info() -> &'static TypeInfo {
        static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
        CELL.get_or_set(|| {
            TypeInfo::TupleStruct(TupleStructInfo::new::<Self>(&[UnnamedField::new::<()>(0)]))
        })
    }
}

impl TupleStruct for AnimationTriggerData {
    fn field(&self, index: usize) -> Option<&dyn PartialReflect> {
        match index {
            0 => Some(self.0.as_partial_reflect()),
            _ => None,
        }
    }

    fn field_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        match index {
            0 => Some(self.0.as_partial_reflect_mut()),
            _ => None,
        }
    }

    fn field_len(&self) -> usize {
        1
    }

    fn iter_fields(&self) -> TupleStructFieldIter {
        TupleStructFieldIter::new(self)
    }

    fn clone_dynamic(&self) -> DynamicTupleStruct {
        DynamicTupleStruct::from_iter([PartialReflect::clone_value(&*self.0)])
    }
}

impl PartialReflect for AnimationTriggerData {
    #[inline]
    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        Some(<Self as Typed>::type_info())
    }

    #[inline]
    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
        self
    }

    #[inline]
    fn as_partial_reflect(&self) -> &dyn PartialReflect {
        self
    }

    #[inline]
    fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        self
    }

    fn try_into_reflect(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
        Ok(self)
    }

    #[inline]
    fn try_as_reflect(&self) -> Option<&dyn Reflect> {
        Some(self)
    }

    #[inline]
    fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> {
        Some(self)
    }

    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
        if let ReflectRef::TupleStruct(struct_value) = value.reflect_ref() {
            for (i, value) in struct_value.iter_fields().enumerate() {
                if let Some(v) = self.field_mut(i) {
                    v.try_apply(value)?;
                }
            }
        } else {
            return Err(ApplyError::MismatchedKinds {
                from_kind: value.reflect_kind(),
                to_kind: ReflectKind::TupleStruct,
            });
        }
        Ok(())
    }

    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::TupleStruct(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::TupleStruct(self)
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::TupleStruct(self)
    }

    fn clone_value(&self) -> Box<dyn PartialReflect> {
        Box::new(Clone::clone(self))
    }
}

impl Reflect for AnimationTriggerData {
    #[inline]
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    #[inline]
    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
        self
    }

    #[inline]
    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    #[inline]
    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    #[inline]
    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }
}

impl FromReflect for AnimationTriggerData {
    fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
        Some(reflect.try_downcast_ref::<AnimationTriggerData>()?.clone())
    }
}
