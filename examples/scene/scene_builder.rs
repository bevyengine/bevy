//! This example illustrates building fine-tuned scenes from a world.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Make sure all components and resources you want to use in your scene are registered,
        // and also register the `ReflectComponent` and `ReflectResource` type data, respectively.
        .register_type::<Unit>()
        .register_type::<Magic>()
        .register_type::<Melee>()
        .register_type::<Archery>()
        .register_type::<Health>()
        .register_type::<CastleHealth>()
        .register_type::<TotalScore>()
        .add_systems(Startup, (initialize_world, infotext_system))
        .add_systems(Update, create_snapshot.run_if(run_once()))
        .run();
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct Unit;

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct Magic(u8);

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct Melee(u8);

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct Archery(u8);

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct Health(u16);

#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
struct CastleHealth(u16);

#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
struct TotalScore(u16);

fn initialize_world(mut commands: Commands) {
    commands.spawn((Unit, Health(100), TransformBundle::default()));
    commands.spawn((Unit, Melee(22), Health(150), TransformBundle::default()));
    commands.spawn((Unit, Magic(16), Health(100), TransformBundle::default()));
    commands.spawn((Unit, Magic(10), Health(75), TransformBundle::default()));
    commands.spawn((Unit, Archery(12), Health(85), TransformBundle::default()));

    commands.insert_resource(CastleHealth(1000));
    commands.insert_resource(TotalScore(0));
}

fn create_snapshot(world: &mut World) {
    // 1. The simplest thing we can do is extract a single entity.
    {
        let entity = world.spawn((Unit, Archery(1))).id();

        let mut builder = DynamicSceneBuilder::from_world(world);

        // By default, this will extract all components that registered `ReflectComponent` from the entity.
        builder.extract_entity(entity);

        print_scene("Single Entity", world, builder.build());
    }

    // 2. We can also use a query (or any `Entity` iterator) to extract multiple entities.
    //    Here, we'll extract all entities that contain a `Unit` and `Magic` component.
    {
        let mut query = world.query_filtered::<Entity, (With<Unit>, With<Magic>)>();

        // Note: `DynamicSceneBuilder` holds onto a reference to the world.
        //       This is why we had to create `query` first, since it requires `&mut World`.
        let mut builder = DynamicSceneBuilder::from_world(world);

        // Any iterator that yields `Entity` can be used, even a query!
        builder.extract_entities(query.iter(world));

        print_scene("Entity Iterator", world, builder.build());
    }

    // 3. The builder can also specify a filter for more control over what gets extracted.
    //    Let's filter that same query to only include the `Magic` data.
    {
        let mut query = world.query_filtered::<Entity, (With<Unit>, With<Magic>)>();

        let mut builder = DynamicSceneBuilder::from_world(world);

        // Set up our filters:
        builder.allow::<Magic>();
        // Alternatively (if we know exactly what we want to exclude):
        // builder
        //     .deny::<Unit>()
        //     .deny::<Melee>()
        //     .deny::<Archery>()
        //     .deny::<Transform>();

        builder.extract_entities(query.iter(world));

        print_scene("Filtered", world, builder.build());
    }

    // 4. Resources can also be extracted.
    //    There are a lot of resources in a default Bevy world (and some that aren't serializable),
    //    so let's employ the same filtering technique as above.
    {
        let mut builder = DynamicSceneBuilder::from_world(world);

        // Set up our filters:
        builder
            .allow_resource::<CastleHealth>()
            .allow_resource::<TotalScore>();

        builder.extract_resources();

        print_scene("Resources", world, builder.build());
    }

    // 5. Extraction can only be performed once per entity/resource.
    //    The first extraction will use whatever filter is set at the time.
    {
        let entity = world.spawn((Unit, Health(100), Melee(16))).id();

        let mut builder = DynamicSceneBuilder::from_world(world);

        // Extract the entity (again, this can only be done once):
        builder.allow::<Melee>().extract_entity(entity);

        // Let's attempt to extract the same entity, but with a modified filter:
        builder.allow::<Health>().extract_entity(entity);

        // Note that we only captured the `Melee` component in the built scene.
        print_scene("Duplicate Extraction", world, builder.build());
    }
}

fn print_scene(title: &str, world: &World, scene: DynamicScene) {
    let type_registry = world.resource::<AppTypeRegistry>();
    let serialized_scene = scene.serialize_ron(type_registry).unwrap();

    info!(
        "==============[ {} ]==============\n{}",
        title, serialized_scene
    );
}

// This is only necessary for the info message in the UI. See examples/ui/text.rs for a standalone
// text example.
fn infotext_system(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn(
        TextBundle::from_section(
            "Nothing to see in this window! Check the console output!",
            TextStyle {
                font_size: 50.0,
                color: Color::WHITE,
                ..default()
            },
        )
        .with_style(Style {
            align_self: AlignSelf::FlexEnd,
            ..default()
        }),
    );
}
