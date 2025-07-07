//! Disabling entities is a powerful feature that allows you to hide entities from the ECS without deleting them.
//!
//! This can be useful for implementing features like "sleeping" objects that are offscreen
//! or managing networked entities.
//!
//! While disabling entities *will* make them invisible,
//! that's not its primary purpose!
//! [`Visibility`](bevy::prelude::Visibility) should be used to hide entities;
//! disabled entities are skipped entirely, which can lead to subtle bugs.
//!
//! # Default query filters
//!
//! Under the hood, Bevy uses a "default query filter" that skips entities with the
//! the [`Disabled`] component.
//! These filters act as a by-default exclusion list for all queries,
//! and can be bypassed by explicitly including these components in your queries.
//! For example, `Query<&A, With<Disabled>`, `Query<(Entity, Has<Disabled>>)` or
//! `Query<&A, Or<(With<Disabled>, With<B>)>>` will include disabled entities.

use bevy::ecs::entity_disabling::Disabled;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MeshPickingPlugin))
        .add_observer(disable_entities_on_click)
        .add_systems(
            Update,
            (list_all_named_entities, reenable_entities_on_space),
        )
        .add_systems(Startup, (setup_scene, display_instructions))
        .run();
}

#[derive(Component)]
struct DisableOnClick;

fn disable_entities_on_click(
    trigger: On<Pointer<Click>>,
    valid_query: Query<&DisableOnClick>,
    mut commands: Commands,
) {
    let clicked_entity = trigger.target();
    // Windows and text are entities and can be clicked!
    // We definitely don't want to disable the window itself,
    // because that would cause the app to close!
    if valid_query.contains(clicked_entity) {
        // Just add the `Disabled` component to the entity to disable it.
        // Note that the `Disabled` component is *only* added to the entity,
        // its children are not affected.
        commands.entity(clicked_entity).insert(Disabled);
    }
}

#[derive(Component)]
struct EntityNameText;

// The query here will not find entities with the `Disabled` component,
// because it does not explicitly include it.
fn list_all_named_entities(
    query: Query<&Name>,
    mut name_text_query: Query<&mut Text, With<EntityNameText>>,
    mut commands: Commands,
) {
    let mut text_string = String::from("Named entities found:\n");
    // Query iteration order is not guaranteed, so we sort the names
    // to ensure the output is consistent.
    for name in query.iter().sort::<&Name>() {
        text_string.push_str(&format!("{name:?}\n"));
    }

    if let Ok(mut text) = name_text_query.single_mut() {
        *text = Text::new(text_string);
    } else {
        commands.spawn((
            EntityNameText,
            Text::default(),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                right: Val::Px(12.0),
                ..default()
            },
        ));
    }
}

fn reenable_entities_on_space(
    mut commands: Commands,
    // This query can find disabled entities,
    // because it explicitly includes the `Disabled` component.
    disabled_entities: Query<Entity, With<Disabled>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.just_pressed(KeyCode::Space) {
        for entity in disabled_entities.iter() {
            // To re-enable an entity, just remove the `Disabled` component.
            commands.entity(entity).remove::<Disabled>();
        }
    }
}

const X_EXTENT: f32 = 900.;

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);

    let named_shapes = [
        (Name::new("Annulus"), meshes.add(Annulus::new(25.0, 50.0))),
        (
            Name::new("Bestagon"),
            meshes.add(RegularPolygon::new(50.0, 6)),
        ),
        (Name::new("Rhombus"), meshes.add(Rhombus::new(75.0, 100.0))),
    ];
    let num_shapes = named_shapes.len();

    for (i, (name, shape)) in named_shapes.into_iter().enumerate() {
        // Distribute colors evenly across the rainbow.
        let color = Color::hsl(360. * i as f32 / num_shapes as f32, 0.95, 0.7);

        commands.spawn((
            name,
            DisableOnClick,
            Mesh2d(shape),
            MeshMaterial2d(materials.add(color)),
            Transform::from_xyz(
                // Distribute shapes from -X_EXTENT/2 to +X_EXTENT/2.
                -X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * X_EXTENT,
                0.0,
                0.0,
            ),
        ));
    }
}

fn display_instructions(mut commands: Commands) {
    commands.spawn((
        Text::new(
            "Click an entity to disable it.\n\nPress Space to re-enable all disabled entities.",
        ),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}
