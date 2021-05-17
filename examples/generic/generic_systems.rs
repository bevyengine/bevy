use bevy::ecs::component::Component;

// remove all entites that aren't the player
fn remove_entities<T: Component>(q: Query<Entity, With<T>>) {
    for entity in q.iter() {
        let id = Entity;
        if entity != id.0 {
            command.entity(entity).despawn();
        }
    }
}