use bevy_ecs::component::Component;
use bevy_math::{Quat, Vec3};

/// Describes the relationship between [`Parent`](crate::Parent) and [`Children`](crate::Children) by mapping [`Transform`](crate::Transform)s
/// 
/// Add to a child entity to control how the Parent's transform affects the Child's
/// 
/// `None` describes no relation, compare to `Some(|_| ())` which maps the parent to child 1:1
/// 
/// The [`Default`] implementation is a 1:1 relation, the same result as if not added to a child entity
#[derive(Component, Clone, Copy)]
pub struct Relation {
    pub translation: Option<fn(&mut Vec3)>,
    pub rotation: Option<fn(&mut Quat)>,
    pub scale: Option<fn(&mut Vec3)>
}

impl Default for Relation {
    fn default() -> Self {
        Self {
            translation: Some(|_| ()),
            rotation: Some(|_| ()),
            scale: Some(|_| ()) 
        }
    }
}