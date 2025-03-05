use crate::entity::{Entity, VisitEntities, VisitEntitiesMut};
use bevy_reflect::{FromReflect, FromType, PartialReflect};

/// For a reflected value, apply an operation to all contained entities.
///
/// See [`VisitEntities`] for more details.
#[derive(Clone)]
pub struct ReflectVisitEntities {
    visit_entities: fn(&dyn PartialReflect, &mut dyn FnMut(Entity)),
}

impl ReflectVisitEntities {
    /// A general method for applying an operation to all entities in a
    /// reflected component.
    pub fn visit_entities(&self, component: &dyn PartialReflect, f: &mut dyn FnMut(Entity)) {
        (self.visit_entities)(component, f);
    }
}

impl<C: FromReflect + VisitEntities> FromType<C> for ReflectVisitEntities {
    fn from_type() -> Self {
        ReflectVisitEntities {
            visit_entities: |component, f| {
                let concrete = C::from_reflect(component).unwrap();
                concrete.visit_entities(f);
            },
        }
    }
}

/// For a reflected value, apply an operation to mutable references to all
/// contained entities.
///
/// See [`VisitEntitiesMut`] for more details.
#[derive(Clone)]
pub struct ReflectVisitEntitiesMut {
    visit_entities_mut: fn(&mut dyn PartialReflect, &mut dyn FnMut(&mut Entity)),
}

impl ReflectVisitEntitiesMut {
    /// A general method for applying an operation to all entities in a
    /// reflected component.
    pub fn visit_entities(
        &self,
        component: &mut dyn PartialReflect,
        f: &mut dyn FnMut(&mut Entity),
    ) {
        (self.visit_entities_mut)(component, f);
    }
}

impl<C: FromReflect + VisitEntitiesMut> FromType<C> for ReflectVisitEntitiesMut {
    fn from_type() -> Self {
        ReflectVisitEntitiesMut {
            visit_entities_mut: |component, f| {
                let mut concrete = C::from_reflect(component).unwrap();
                concrete.visit_entities_mut(f);
                component.apply(&concrete);
            },
        }
    }
}
