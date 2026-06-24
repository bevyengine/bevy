//! Demonstrates how to modify component values in a type-erased way,
//! using Bevy's runtime type reflection functionality
//! to operate generically over any component type or shape of data.
//!
//! This path is useful when building tools like inspectors or for integrating scripting languages,
//! where you want to modify component values without knowing the type at compile time.
//! It will be *much* slower than modifying values directly,
//! so it should only be used when you have no other choice,
//! or when flexibility is the most important consideration.

use bevy::{prelude::*, reflect::ReflectMut};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<SelectedComponent>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (select_component_to_modify, modify_selected_component),
        )
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    commands.spawn(Sprite {
        image: asset_server.load("branding/bevy_logo_dark.png"),
        ..Default::default()
    });

    let instructions = "\
Press 'T' to select Transform component's y-translation for modification
Press 'S' to select Sprite component's alpha value for modification
Press 'Up Arrow' to increase the selected component's value
Press 'Down Arrow' to decrease the selected component's value"
        .to_string();

    commands.spawn((
        Text::new(instructions),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

// The component type to modify should generally be selected via UI.
#[derive(Resource, Default, Clone)]
enum SelectedComponent {
    #[default]
    Transform,
    Sprite,
}

// Quickly select which component to modify via keyboard input,
// avoiding the need for a full UI in this example.
fn select_component_to_modify(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut selected: ResMut<SelectedComponent>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyT) {
        *selected = SelectedComponent::Transform;
        info!("Selected Transform component for modification");
    } else if keyboard_input.just_pressed(KeyCode::KeyS) {
        *selected = SelectedComponent::Sprite;
        info!("Selected Sprite component for modification");
    }
}

/// This function demonstrates the core logic of modifying a component value via reflection.
///
/// Because we're operating over *any* component type,
/// we require full &mut access to [`World`].
///
/// To mutate a component value via reflection:
/// 1. Determine the entity whose component you want to modify.
/// 2. Determine the `TypeId` of the component type to modify. A real tool starts from a
///    type *name*, so we resolve it to a `TypeId` through the [`AppTypeRegistry`].
/// 3. Get a mutable reference to the component as [`&mut dyn Reflect`](Reflect).
/// 4. Construct a replacement value based on the existing value.
/// 5. Modify the existing value using [`PartialReflect::apply`] or its relatives.
///
/// If you want to access a specific field on a component, between steps 3 and 4 you need to:
///
/// 1. Determine the shape of the type using reflection, by converting that to a [`ReflectMut`] object.
/// 2. Find the field(s) you want to modify by walking the tree of type metadata.
/// 3. Downcast each field to a concrete type (e.g. with `try_downcast_ref`) to read its value.
fn modify_selected_component(world: &mut World) {
    // We're using keyboard input to trigger modifications for simplicity.
    let button_input = world.resource::<ButtonInput<KeyCode>>();
    let direction_of_modification = if button_input.pressed(KeyCode::ArrowUp) {
        1.0
    } else if button_input.pressed(KeyCode::ArrowDown) {
        -1.0
    } else {
        return; // No modification requested
    };

    let selected = world.resource::<SelectedComponent>().clone();

    let mut sprite_query = world.query_filtered::<Entity, With<Sprite>>();

    // This entity should generally be gathered via UI selection in a real application
    let entity = sprite_query.iter(world).next().unwrap();

    // We could cheat and use `TypeId::of::<T>()` to get the type ID of a known type,
    // but real applications identifies types by name (from a UI dropdown, text entry or a script)
    let type_name = match selected {
        SelectedComponent::Transform => "Transform",
        SelectedComponent::Sprite => "Sprite",
    };

    // Then, we need to use a type registry to resolve the type name to a `TypeId`.
    // Types are (for the most part) registered automatically by Bevy,
    // but you can also register your own types using .register_type.
    // Generic types always need to be registered manually unfortunately;
    // if a type is not showing up in your tool, check if that's the problem.
    let app_registry = world.resource::<AppTypeRegistry>().clone();
    let type_id = app_registry
        .read()
        .get_with_short_type_path(type_name)
        .expect("type was not registered, or its short name was ambiguous")
        .type_id();

    let mut dynamic_mut = world.get_reflect_mut(entity, type_id).unwrap();

    match selected {
        // Downcasting is the easy path:
        // if you happen to know the type,
        // you can downcast and modify directly.
        // The problem is that each of these paths would need to be hard-coded (or use code-gen),
        // largely defeating the purpose of using reflection in the first place.
        SelectedComponent::Sprite => {
            // Make sure that the type matches the component type you requested to modify.
            // In a real project, you would want to handle this gracefully.
            // Downcasting converts the value *directly* into a specified concrete type,
            // allowing you to escape back into faster, strongly-typed code.
            let downcasted = dynamic_mut.downcast_mut::<Sprite>().unwrap();
            // Be careful not to modify a copy of the color!
            let color = &mut downcasted.color;

            let new_alpha = (color.alpha() + 0.01 * direction_of_modification).clamp(0.0, 1.0);
            color.set_alpha(new_alpha);
        }
        // More realistically, we have to walk the reflected type info to find fields to modify.
        SelectedComponent::Transform => {
            let reflect_mut = dynamic_mut.reflect_mut();
            // In the generic case, we would want to match on the `ReflectMut` variants
            let ReflectMut::Struct(struct_mut) = reflect_mut else {
                error!("Expected the Transform component type to be a struct");
                return;
            };

            // Get the `translation` field
            let translation_field = struct_mut.field_mut("translation").unwrap();

            // Now, repeat the process to get the `y` field of the `translation` Vec3
            let ReflectMut::Struct(translation_struct) = translation_field.reflect_mut() else {
                error!("Expected the translation field to be a struct");
                return;
            };

            let y_field = translation_struct.field_mut("y").unwrap();

            // Convert the field to a concrete type to read the current value
            let current_y = y_field.try_downcast_ref::<f32>().unwrap();
            let new_y = current_y + 10.0 * direction_of_modification;

            // Set the new value using reflection
            // Unlike downcasting, apply and similar methods stay type-erased the entire time,
            // so you can work on data that doesn't *have* a concrete Rust type.
            y_field.apply(&new_y);
        }
    }
}
