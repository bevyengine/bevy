//! Shows how entity blueprints can be used to create abstractions for heirarchys of entities
//! This is useful when a component depends on a child components state to know how to function

use bevy::{ecs::blueprint::EntityBlueprint, prelude::*};

#[derive(Component)]
pub struct MainComponent {
    pub child: Entity,
    pub other_child: Entity,
}

#[derive(Component)]
pub struct ChildComponent {
    pub info: f32,
}

pub struct ExampleBlueprint {
    pub child_info: f32,
    pub other_child_info: f32,
}

impl EntityBlueprint for ExampleBlueprint {
    fn build(self, entity: &mut bevy::ecs::system::EntityCommands) {
        let child = entity
            .commands()
            .spawn(ChildComponent {
                info: self.child_info,
            })
            .id();

        let other_child = entity
            .commands()
            .spawn(ChildComponent {
                info: self.other_child_info,
            })
            .id();

        entity
            .insert(MainComponent { child, other_child })
            // stored in main component as a direct link to entity,
            // so it doesnt necessarily need to be a child
            // .add_child(child)
            .add_child(other_child);
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(print_example_sum)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn_blueprint(ExampleBlueprint {
        child_info: 5.,
        other_child_info: 10.,
    });
}

fn print_example_sum(query: Query<&MainComponent>, child_query: Query<&ChildComponent>) {
    for component in query.iter() {
        let Ok(child) = child_query.get(component.child) else {
            return
        };
        let Ok(other_child) = child_query.get(component.other_child) else {
            return
        };
        println!(
            "Sum of linked object info: {:?}",
            child.info + other_child.info
        );
    }
}
