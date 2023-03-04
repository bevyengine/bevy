#![allow(clippy::type_complexity)]

use bevy_app::App;
use bevy_ecs::{
    prelude::{Component, Entity, ResMut},
    query::With,
    schedule::IntoSystemConfig,
    system::{Commands, In, IntoPipeSystem, Query},
};
use bevy_entropy::prelude::*;
use rand::prelude::{IteratorRandom, Rng};
use rand_chacha::ChaCha8Rng;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Enemy;

#[derive(Component, PartialEq, Eq)]
enum Kind {
    Player,
    Enemy,
}

#[derive(Component)]
struct Name(pub String);

#[derive(Component)]
struct Attack {
    max: f32,
    min: f32,
}

#[derive(Component)]
struct Defense {
    dodge: f64,
    armor: f32,
}

#[derive(Component)]
struct Buff {
    effect: f32,
    chance: f64,
}

#[derive(Component)]
struct Health {
    amount: f32,
}

fn main() {
    App::new()
        .add_plugin(EntropyPlugin::<ChaCha8Rng>::new().with_seed([1; 32]))
        .add_startup_systems((setup_player, setup_enemies.after(setup_player)))
        .add_system(determine_attack_order.pipe(attack_turn))
        .add_system(buff_entities.after(attack_turn))
        .run();
}

fn setup_player(mut commands: Commands, mut rng: ResMut<GlobalEntropy<ChaCha8Rng>>) {
    commands.spawn((
        Kind::Player,
        Name("Player".into()),
        Attack {
            max: 10.0,
            min: 2.0,
        },
        Defense {
            dodge: 0.25,
            armor: 3.0,
        },
        Buff {
            effect: 5.0,
            chance: 0.5,
        },
        Health { amount: 50.0 },
        // Forking from the global instance creates a random, but deterministic
        // seed for the component, making it hard to guess yet still have a
        // deterministic output
        EntropyComponent::from(&mut rng),
    ));
}

fn setup_enemies(mut commands: Commands, mut rng: ResMut<GlobalEntropy<ChaCha8Rng>>) {
    for i in 1..=2 {
        commands.spawn((
            Kind::Enemy,
            Name(format!("Goblin {i}")),
            Attack { max: 8.0, min: 1.0 },
            Defense {
                dodge: 0.2,
                armor: 2.5,
            },
            Buff {
                effect: 5.0,
                chance: 0.25,
            },
            Health { amount: 20.0 },
            // Forking from the global instance creates a random, but deterministic
            // seed for the component, making it hard to guess yet still have a
            // deterministic output
            EntropyComponent::from(&mut rng),
        ));
    }
}

fn determine_attack_order(
    mut q_entities: Query<(Entity, &mut EntropyComponent<ChaCha8Rng>), With<Kind>>,
) -> Vec<Entity> {
    // No matter the order of entities in the query, because they have their own RNG instance,
    // it will always result in a deterministic output due to being seeded from a single global
    // RNG instance with a chosen seed.
    let mut entities: Vec<_> = q_entities
        .iter_mut()
        .map(|mut entity| (entity.1.gen::<u32>(), entity))
        .collect();

    entities.sort_by_key(|k| k.0);

    entities.iter_mut().map(|(_, entity)| entity.0).collect()
}

fn attack_turn(
    In(attack_order): In<Vec<Entity>>,
    mut q_entities: Query<(
        Entity,
        &Kind,
        &Attack,
        &Defense,
        &Name,
        &mut Health,
        &mut EntropyComponent<ChaCha8Rng>,
    )>,
) {
    // Establish list of enemy entities for player to attack
    let enemies: Vec<_> = q_entities
        .iter()
        .filter_map(|entity| entity.1.eq(&Kind::Enemy).then_some(entity.0))
        .collect();

    // Get the Player entity for the enemies to target
    let player = q_entities
        .iter()
        .find_map(|entity| entity.1.eq(&Kind::Player).then_some(entity.0))
        .unwrap();

    // We've created a sorted attack order from another system, so this should always be deterministic.
    for entity in attack_order {
        // Calculate the target and the amount of damage to attempt to apply to the target.
        let (target, attack_damage, attacker) = {
            let (_, attacker, attack, _, name, _, mut a_rng) = q_entities.get_mut(entity).unwrap();

            let attack_damage = a_rng.gen_range(attack.min..=attack.max);

            let target = if attacker == &Kind::Player {
                enemies.iter().choose(a_rng.as_mut()).copied().unwrap()
            } else {
                player
            };

            (target, attack_damage, name.0.clone())
        };

        // Calculate the defense of the target for mitigating the damage.
        let (_, _, _, defense, defender, mut hp, mut d_rng) = q_entities.get_mut(target).unwrap();

        // Will they dodge the attack?
        if d_rng.gen_bool(defense.dodge) {
            println!("{} dodged {}'s attack!", defender.0, attacker);
        } else {
            let damage_taken = (attack_damage - defense.armor).clamp(0.0, f32::MAX);

            hp.amount = (hp.amount - damage_taken).clamp(0.0, f32::MAX);

            println!(
                "{} took {} damage from {}",
                defender.0, damage_taken, attacker
            );
        }
    }
}

fn buff_entities(
    mut q_entities: Query<
        (&Name, &Buff, &mut Health, &mut EntropyComponent<ChaCha8Rng>),
        With<Kind>,
    >,
) {
    // Query iteration order is not stable, but entities having their own RNG source side-steps this
    // completely, so the result is always deterministic.
    for (name, buff, mut hp, mut rng) in q_entities.iter_mut() {
        if rng.gen_bool(buff.chance) {
            hp.amount += buff.effect;

            println!("{} buffed their health by {} points!", name.0, buff.effect);
        }
    }
}
