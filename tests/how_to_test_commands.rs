use bevy::ecs::system::Command;
use bevy::prelude::*;

struct Request;

#[derive(Component)]
struct Requests(Vec<Request>);

impl Requests {
    fn new(request: Request) -> Self {
        Self([request].into())
    }

    fn add(&mut self, request: Request) {
        self.0.push(request);
    }

    fn len(&self) -> usize {
        self.0.len()
    }
}

struct InsertOrAddRequest(pub Entity, pub Request);

impl Command for InsertOrAddRequest {
    fn write(self, world: &mut World) {
        let Self(entity, request) = self;
        if let Some(mut requests) = world.get_mut::<Requests>(entity) {
            requests.add(request);
        } else {
            world.entity_mut(entity).insert(Requests::new(request));
        }
    }
}

#[test]
fn add_request() {
    use bevy::ecs::system::ApplyCommands;

    let mut world = World::new();
    let entity = world.spawn_empty().id();
    world.apply_commands(|_, mut commands| {
        commands.add(InsertOrAddRequest(entity, Request));
        commands.add(InsertOrAddRequest(entity, Request));
    });

    let requests = world.get::<Requests>(entity);

    assert_eq!(
        requests.expect("entity must have requests").len(),
        2,
        "entity must have exactly 2 requests"
    );
}
