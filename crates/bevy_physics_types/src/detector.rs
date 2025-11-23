use bevy_ecs::{component::Component, entity::{Entities, EntityHashSet}};

/// detects things entering it using its collider.
#[derive(Component)]
struct Detector;

/// keeps track of things
#[derive(Component)]
struct TrackInside(EntityHashSet);
