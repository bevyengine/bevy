//! Demonstrates the component constraint system.
//!
//! Constraints allow declaring rules about which components can coexist
//! in the same archetype. When a constraint is violated, the operation
//! is rejected (RESTRICT) and the entity stays in its previous state.
//!
//! Available constraint primitives:
//! - `require(T)`: component T must be present
//! - `forbid(T)`: component T must NOT be present
//! - `and(...)`: all sub-constraints must hold
//! - `or(...)`: at least one sub-constraint must hold
//! - `not(...)`: negates a sub-constraint
//! - `only(T1, T2, ...)`: only these components (plus self) are allowed

use bevy::{
    log::{self, LogPlugin},
    prelude::*,
};

#[derive(Component, Default, Debug)]
struct Health;

#[derive(Component, Default, Debug)]
struct Mana;

#[derive(Component, Default, Debug)]
struct Armor;

#[derive(Component, Default, Debug)]
struct Enemy;

#[derive(Component, Default, Debug)]
struct Scroll;

/// Player requires Health - cannot exist without it.
#[derive(Component, Debug)]
#[constraint(require(Health))]
struct Player;

/// Ally forbids Enemy - they cannot coexist on the same entity.
#[derive(Component, Default, Debug)]
#[constraint(forbid(Enemy))]
struct Ally;

/// Caster requires either Mana or Scroll.
#[derive(Component, Debug)]
#[constraint(or(require(Mana), require(Scroll)))]
struct Caster;

/// Warrior can only coexist with Health and Armor
#[derive(Component, Debug)]
#[constraint(only(Health, Armor))]
struct Warrior;

/// Knight combines both: requires Health, and only allows Health + Armor.
#[derive(Component, Debug)]
#[constraint(require(Health))]
#[constraint(only(Health, Armor))]
struct Knight;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(LogPlugin::default())
        .add_systems(Startup, demo)
        .run();
}

fn demo(mut commands: Commands, mut exit: MessageWriter<AppExit>) {
    println!("=== add \"--feature debug\" to see logger output ===\n");
    log::info!("\n=== require constraint ===");

    // OK: Player + Health satisfies require(Health)
    commands.spawn((Player, Health));
    log::info!("Spawning Player + Health...");

    // FAIL: Player alone - missing Health
    commands.spawn(Player);
    log::info!("Spawning Player alone (should be rejected)...");

    log::info!("\n=== forbid constraint ===");

    // OK: Ally without Enemy
    commands.spawn(Ally);
    log::info!("Spawning Ally alone...");

    // FAIL: Ally + Enemy - forbidden
    commands.spawn((Ally, Enemy));
    log::info!("Spawning Ally + Enemy (should be rejected)...");

    log::info!("\n=== or constraint ===");

    // OK: Caster + Mana
    commands.spawn((Caster, Mana));
    log::info!("Spawning Caster + Mana...");

    // OK: Caster + Scroll
    commands.spawn((Caster, Scroll));
    log::info!("Spawning Caster + Scroll...");

    // FAIL: Caster alone - neither Mana nor Scroll
    commands.spawn(Caster);
    log::info!("Spawning Caster alone (should be rejected)...");

    log::info!("\n=== only constraint ===");

    // OK: Warrior + Health + Armor - all in whitelist
    commands.spawn((Warrior, Health, Armor));
    log::info!("Spawning Warrior + Health + Armor...");

    // FAIL: Warrior + Health + Enemy - Enemy not in whitelist
    commands.spawn((Warrior, Health, Enemy));
    log::info!("Spawning Warrior + Health + Enemy (should be rejected)...");

    log::info!("\n=== only + require combined ===");

    // OK: Knight + Health
    commands.spawn((Knight, Health));
    log::info!("Spawning Knight + Health...");

    // FAIL: Knight alone - missing required Health
    commands.spawn(Knight);
    log::info!("Spawning Knight alone (should be rejected)...");

    // FAIL: Knight + Health + Enemy - Enemy violates only
    commands.spawn((Knight, Health, Enemy));
    log::info!("Spawning Knight + Health + Enemy (should be rejected)...");

    exit.write(AppExit::Success);
}
