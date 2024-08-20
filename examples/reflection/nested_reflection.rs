//! This example demonstrates how reflection trait objects can be nested in other reflected types
//! using remote reflection.

use bevy::prelude::*;

fn main() {
    // Bevy's reflection crate relies heavily on the `dyn Reflect` trait object.
    // This allows the compile-time name of a type to be "erased" and passed around at runtime,
    // most often as a `Box<dyn Reflect>`.
    let _: Box<dyn Reflect> = Box::new(String::from("Hello, World!"));

    // However, you'll notice that `Box<dyn Reflect>` itself doesn't implement `Reflect`.
    // This makes it impossible to use `Box<dyn Reflect>` as a field in a struct that derives `Reflect`.
    // ```
    // #[derive(Reflect)]
    // struct MyType {
    //     field: Box<dyn Reflect>, // <- Compile Error
    // }
    // ```
    // This is because it would be too easy to accidentally box a `Reflect` type,
    // then accidentally box it again, and again, and so on.
    // So instead, `bevy_reflect` exposes a `ReflectBox` type which can be used
    // as a remote wrapper around a `Box<dyn Reflect>` (or `Box<dyn PartialReflect>`).
    //
    // For example, let's say we want to define some equipment for a player.
    // We don't know what kind of equipment the player will have at compile time,
    // so we want to store it as a `Box<dyn Reflect>`.
    // To do this, we first need to derive `Reflect` for our `Player`.
    #[derive(Reflect)]
    // Next, we need to opt out of deriving `FromReflect` since `Box<dyn Reflect>`
    // has no knowledge of `FromReflect`.
    #[reflect(from_reflect = false)]
    struct Player {
        // Now we can use remote reflection to tell `Reflect` how to reflect our `Box<dyn Reflect>`.
        #[reflect(remote = bevy::reflect::boxed::ReflectBox<dyn Reflect>)]
        equipment: Box<dyn Reflect>,
    }

    // Now we can use any type that implements `Reflect` as equipment for our player.
    let equipment: Box<dyn Reflect> = Box::new(String::from("Sword"));
    let mut player: Box<dyn Struct> = Box::new(Player { equipment });

    // We can also use reflection to modify our player's equipment.
    let equipment = player.field_mut("equipment").unwrap();
    equipment.try_apply(&String::from("Shield")).unwrap();
    assert!(player
        .reflect_partial_eq(&Player {
            equipment: Box::new(String::from("Shield")),
        })
        .unwrap_or_default());
}
