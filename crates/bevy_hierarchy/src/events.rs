use bevy_ecs::prelude::Entity;

#[derive(Clone)]
pub struct ChildAdded {
    pub parent: Entity,
    pub child: Entity,
}

#[derive(Clone)]
pub struct ChildRemoved {
    pub parent: Entity,
    pub child: Entity,
}

#[derive(Clone)]
pub struct ChildMoved {
    pub child: Entity,
    pub previous_parent: Entity,
    pub new_parent: Entity,
}
