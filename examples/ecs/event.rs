//! This example shows how to send, mutate, and receive, events. It also demonstrates
//! how to control system ordering so that events are processed in a specific order.
//! It does this by simulating a damage over time effect that you might find in a game.

use bevy::prelude::*;

// In order to send or receive events first you must define them
// This event should be sent when something attempts to deal damage to another entity.
#[derive(BufferedEvent, Debug)]
struct DealDamage {
    pub amount: i32,
}

// This event should be sent when an entity receives damage.
#[derive(BufferedEvent, Debug, Default)]
struct DamageReceived;

// This event should be sent when an entity blocks damage with armor.
#[derive(BufferedEvent, Debug, Default)]
struct ArmorBlockedDamage;

// This resource represents a timer used to determine when to deal damage
// By default it repeats once per second
#[derive(Resource, Deref, DerefMut)]
struct DamageTimer(pub Timer);

impl Default for DamageTimer {
    fn default() -> Self {
        DamageTimer(Timer::from_seconds(1.0, TimerMode::Repeating))
    }
}

// Next we define systems that send, mutate, and receive events
// This system reads 'DamageTimer', updates it, then sends a 'DealDamage' event
// if the timer has finished.
//
// Events are sent using an 'EventWriter<T>' by calling 'write' or 'write_default'.
// The 'write_default' method will send the event with the default value if the event
// has a 'Default' implementation.
fn deal_damage_over_time(
    time: Res<Time>,
    mut state: ResMut<DamageTimer>,
    mut events: EventWriter<DealDamage>,
) {
    if state.tick(time.delta()).is_finished() {
        // Events can be sent with 'write' and constructed just like any other object.
        events.write(DealDamage { amount: 10 });
    }
}

// This system mutates the 'DealDamage' events to apply some armor value
// It also sends an 'ArmorBlockedDamage' event if the value of 'DealDamage' is zero
//
// Events are mutated using an 'EventMutator<T>' by calling 'read'. This returns an iterator
// over all the &mut T that this system has not read yet. Note, you can have multiple
// 'EventReader', 'EventWriter', and 'EventMutator' in a given system, as long as the types (T) are different.
fn apply_armor_to_damage(
    mut dmg_events: EventMutator<DealDamage>,
    mut armor_events: EventWriter<ArmorBlockedDamage>,
) {
    for event in dmg_events.read() {
        event.amount -= 1;
        if event.amount <= 0 {
            // Zero-sized events can also be sent with 'send'
            armor_events.write(ArmorBlockedDamage);
        }
    }
}

// This system reads 'DealDamage' events and sends 'DamageReceived' if the amount is non-zero
//
// Events are read using an 'EventReader<T>' by calling 'read'. This returns an iterator over all the &T
// that this system has not read yet, and must be 'mut' in order to track which events have been read.
// Again, note you can have multiple 'EventReader', 'EventWriter', and 'EventMutator' in a given system,
// as long as the types (T) are different.
fn apply_damage_to_health(
    mut dmg_events: EventReader<DealDamage>,
    mut rcvd_events: EventWriter<DamageReceived>,
) {
    for event in dmg_events.read() {
        info!("Applying {} damage", event.amount);
        if event.amount > 0 {
            // Events with a 'Default' implementation can be written with 'write_default'
            rcvd_events.write_default();
        }
    }
}

// Finally these two systems read 'DamageReceived' events.
//
// The first system will play a sound.
// The second system will spawn a particle effect.
//
// As before, events are read using an 'EventReader' by calling 'read'. This returns an iterator over all the &T
// that this system has not read yet.
fn play_damage_received_sound(mut dmg_events: EventReader<DamageReceived>) {
    for _ in dmg_events.read() {
        info!("Playing a sound.");
    }
}

// Note that both systems receive the same 'DamageReceived' events. Any number of systems can
// receive the same event type.
fn play_damage_received_particle_effect(mut dmg_events: EventReader<DamageReceived>) {
    for _ in dmg_events.read() {
        info!("Playing particle effect.");
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Events must be added to the app before they can be used
        // using the 'add_event' method
        .add_event::<DealDamage>()
        .add_event::<ArmorBlockedDamage>()
        .add_event::<DamageReceived>()
        .init_resource::<DamageTimer>()
        // As always we must add our systems to the apps schedule.
        // Here we add our systems to the schedule using 'chain()' so that they run in order
        // This ensures that 'apply_armor_to_damage' runs before 'apply_damage_to_health'
        // It also ensures that 'EventWriters' are used before the associated 'EventReaders'
        .add_systems(
            Update,
            (
                deal_damage_over_time,
                apply_armor_to_damage,
                apply_damage_to_health,
            )
                .chain(),
        )
        // These two systems are not guaranteed to run in order, nor are they guaranteed to run
        // after the above chain. They may even run in parallel with each other.
        // This means they may have a one frame delay in processing events compared to the above chain
        // In some instances this is fine. In other cases it can be an issue. See the docs for more information
        .add_systems(
            Update,
            (
                play_damage_received_sound,
                play_damage_received_particle_effect,
            ),
        )
        .run();
}
