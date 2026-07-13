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
Press 'T' to select the Transform component's y-translation for modification
Press 'S' to select the Sprite component's alpha value for modification
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
/// we require full `&mut` access to [`World`].
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
///
/// [`GetPath`], and its cached variant [`ParsedPath`](bevy::reflect::ParsedPath), can be used to simplify this process.
/// For example, calling `.path_mut::<f32>("translation.y")` on a `ReflectMut` object that stores a transform will return a
/// `&mut f32` to the `y` field of a `Vec3` inside a `Transform`.
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
    // but real applications identify types by name (from a UI dropdown, text entry or a script).
    let type_name = match selected {
        // Note that Bevy's native types are registered under their full subcrate paths:
        // `bevy_transform`, not `bevy::transform`, `bevy::prelude`, or `bevy_transform::prelude`.
        // You can use `<T as TypePath>::type_path()` to look this up.
        SelectedComponent::Transform => "bevy_transform::components::transform::Transform",
        SelectedComponent::Sprite => "bevy_sprite::sprite::Sprite",
    };

    // Then, we need to use a type registry to resolve the type name to a `TypeId`.
    // Types are (for the most part) registered automatically by Bevy,
    // but you can also register your own types using `App::register_type`.
    // Generic types always need to be registered manually;
    // if a type is not showing up in your tool, check if that's the problem.
    // You can check which types are registered by calling `TypeRegistry::iter()`,
    // and then use the `Debug` impl for `TypeRegistration` objects to see their names and paths.
    let app_registry = world.resource::<AppTypeRegistry>().clone();
    let type_id = app_registry
        .read()
        .get_with_type_path(type_name)
        .expect("Type was not registered, or its full path was ambiguous")
        .type_id();

    let mut reflected_component: Mut<dyn Reflect> = world.get_reflect_mut(entity, type_id).unwrap();

    match selected {
        // Downcasting is the easy path:
        // if you happen to know the type, you can downcast and modify directly.
        // The problem is that each of these paths would need to be hard-coded (or rely on extensive code-gen),
        // largely defeating the purpose of using reflection in the first place.
        SelectedComponent::Sprite => {
            // Make sure that the type matches the component type you requested to modify.
            // In a real project, you would want to handle this gracefully.
            // Downcasting converts the value *directly* into a specified concrete type,
            // allowing you to escape back into faster, strongly-typed code.
            let downcast_sprite: &mut Sprite =
                reflected_component.downcast_mut::<Sprite>().unwrap();
            // Be careful not to modify a copy of the color — use `&mut`!
            let color = &mut downcast_sprite.color;

            let new_alpha = (color.alpha() + 0.01 * direction_of_modification).clamp(0.0, 1.0);
            color.set_alpha(new_alpha);
        }
        // This arm demonstrates the more realistic, generic pattern:
        // walking the reflected type info to find fields to modify.
        // The benefit is that we can use these patterns
        // to operate over *any* data based on our knowledge of its shape (recorded using reflection),
        // without needing to know the concrete type at compile time.
        SelectedComponent::Transform => {
            let reflect_mut: ReflectMut<'_> = reflected_component.reflect_mut();
            // In the fully generic case, we would need to match on the `ReflectMut` variants
            // and handle each of the arms exhaustively.
            // `struct_mut` is of type `&mut dyn Struct`, one of a number
            // of traits that encodes the logic of Rust's type system into a runtime representation.
            let ReflectMut::Struct(struct_mut) = reflect_mut else {
                error!("Expected the Transform component type to be a struct");
                return;
            };

            // Get the `translation` field as a `&mut dyn PartialReflect`,
            // which is a type-erased representation of a value that can be modified.
            let translation_field = struct_mut.field_mut("translation").unwrap();

            // Now, we can repeat the process to get the `y` field of the `translation` Vec3
            // In a real application, this would probably be done via a recursive function!
            let ReflectMut::Struct(translation_struct) = translation_field.reflect_mut() else {
                error!("Expected the translation field to be a struct");
                return;
            };

            // We could downcast to an f32 again here, but that would be cheating!
            // How do you generalize this sort of operation, if, for example,
            // you wanted to build a generic inspector that could modify any numeric field of any component type?
            // The solution lies in the way that Bevy can reflect *traits* as well as types,
            // allowing type owners to define and register additional behavior for their types.
            // This data is registered automatically at compile time using an inventory-like solution,
            // and operates on a per-type x per-trait basis,
            // just like ordinary type reflection.
            //
            // We want to increase or decrease the value here,
            // so we need the `AddAssign` trait
            // which are already implemented for f32.
            //
            // But `AddAssign` is not a supertrait of `PartialReflect`!
            // We don't have access to its methods! How could that possibly work?
            //
            // The solution is again to register the compile time information that we want to use at runtime;
            // storing function pointers to the trait methods in the type registry.
            // In order to make this work, we need shadow "reflect" versions of the traits we want to use at runtime.
            // Bevy provides a few of these out of the box, including `ReflectAddAssign` and `ReflectSubAssign`.
            // That's what the `#[reflect(Add)]` attributes that you see scattered about in Bevy's source code are doing:
            // generating implementations of the reflect versions of the traits, so they can later be registered and used at runtime.
            //
            // For more information, see the `type_data` example.
            let y_field: &mut dyn PartialReflect = translation_struct.field_mut("y").unwrap();
            let field_type_id = y_field
                .get_represented_type_info()
                .expect("Found a dynamic type unexpectedly")
                .type_id();

            let add_assign_trait_data = app_registry
                .read()
                .get_type_data::<ReflectAddAssign>(field_type_id)
                .expect("f32 failed to register ReflectAddAssign")
                .clone();

            // We need to operate on the value as a concrete type,
            // so we need to convert it into the more powerful &dyn Reflect type.
            let y_field: &mut dyn Reflect = y_field.try_as_reflect_mut().expect(
                "Found a dynamic type unexpectedly, but we need a concrete type to modify it",
            );

            // We're still cheating a bit here!
            // By doing this we just *assume* that there's an f32 value when trying to determine what to add to the field.
            // We *could* try all of the common numeric types, but that would be slow and non-extensible.
            //
            // In a real workflow, you would want a dedicated trait with additional methods that exposes
            // something like dedicated `increment` and `decrement` methods, which handle the type-specific logic of how to modify the value.
            // Remember to register that trait, and create your own analog of `ReflectAddAssign` for it!
            //
            // We don't do that here to avoid making this example *even more* complicated.
            const MAGNITUDE_OF_MODIFICATION: f32 = 10.;
            let delta = direction_of_modification * MAGNITUDE_OF_MODIFICATION;
            let boxed_delta: Box<dyn PartialReflect> = Box::new(delta);
            add_assign_trait_data
                .add_assign(y_field, boxed_delta)
                .expect("We cheated and know the types match, so this should always succeed.");
        }
    }
}
