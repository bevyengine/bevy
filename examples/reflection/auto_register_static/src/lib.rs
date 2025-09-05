//! Demonstrates how to set up automatic reflect types registration for platforms without `inventory` support
use bevy::prelude::*;

// The type that should be automatically registered.
// All types subject to automatic registration must not be defined in the same crate as `load_type_registrations!``.
// Any `#[derive(Reflect)]` types within the `bin` crate are not guaranteed to be registered automatically.
#[derive(Reflect)]
struct Struct {
    a: i32,
}

mod private {
    mod very_private {
        use bevy::prelude::*;

        // Works with private types too!
        #[derive(Reflect)]
        struct PrivateStruct {
            a: i32,
        }
    }
}

/// This is the main entrypoint, bin just forwards to it.
pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, startup)
        .run();
}

fn startup(reg: Res<AppTypeRegistry>) {
    let registry = reg.read();
    info!(
        "Is `Struct` registered? {}",
        registry.contains(core::any::TypeId::of::<Struct>())
    );
    info!(
        "Type info of `PrivateStruct`: {:?}",
        registry
            .get_with_short_type_path("PrivateStruct")
            .expect("Not registered")
    );
}
