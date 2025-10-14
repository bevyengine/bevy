//! This example shows how to send, mutate, and receive, messages. It also demonstrates
//! how to control system ordering so that messages are processed in a specific order.
//! It does this by simulating a damage over time effect that you might find in a game.

use bevy::prelude::*;

// In order to send or receive messages first you must define them
// This message should be sent when something attempts to deal damage to another entity.
#[derive(Message, Debug)]
struct DealDamage {
    pub amount: i32,
}

// This message should be sent when an entity receives damage.
#[derive(Message, Debug, Default)]
struct DamageReceived;

// This message should be sent when an entity blocks damage with armor.
#[derive(Message, Debug, Default)]
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

// Next we define systems that send, mutate, and receive messages
// This system reads 'DamageTimer', updates it, then sends a 'DealDamage' message
// if the timer has finished.
//
// Messages are sent using an 'MessageWriter<T>' by calling 'write' or 'write_default'.
// The 'write_default' method will send the message with the default value if the message
// has a 'Default' implementation.
fn deal_damage_over_time(
    time: Res<Time>,
    mut state: ResMut<DamageTimer>,
    mut deal_damage_writer: MessageWriter<DealDamage>,
) {
    if state.tick(time.delta()).is_finished() {
        // Messages can be sent with 'write' and constructed just like any other object.
        deal_damage_writer.write(DealDamage { amount: 10 });
    }
}

// This system mutates the 'DealDamage' messages to apply some armor value
// It also sends an 'ArmorBlockedDamage' message if the value of 'DealDamage' is zero
//
// Messages are mutated using an 'MessageMutator<T>' by calling 'read'. This returns an iterator
// over all the &mut T that this system has not read yet. Note, you can have multiple
// 'MessageReader', 'MessageWriter', and 'MessageMutator' in a given system, as long as the types (T) are different.
fn apply_armor_to_damage(
    mut dmg_messages: MessageMutator<DealDamage>,
    mut armor_messages: MessageWriter<ArmorBlockedDamage>,
) {
    for message in dmg_messages.read() {
        message.amount -= 1;
        if message.amount <= 0 {
            // Zero-sized messages can also be sent with 'send'
            armor_messages.write(ArmorBlockedDamage);
        }
    }
}

// This system reads 'DealDamage' messages and sends 'DamageReceived' if the amount is non-zero
//
// Messages are read using an 'MessageReader<T>' by calling 'read'. This returns an iterator over all the &T
// that this system has not read yet, and must be 'mut' in order to track which messages have been read.
// Again, note you can have multiple 'MessageReader', 'MessageWriter', and 'MessageMutator' in a given system,
// as long as the types (T) are different.
fn apply_damage_to_health(
    mut deal_damage_reader: MessageReader<DealDamage>,
    mut damaged_received_writer: MessageWriter<DamageReceived>,
) {
    for deal_damage in deal_damage_reader.read() {
        info!("Applying {} damage", deal_damage.amount);
        if deal_damage.amount > 0 {
            // Messages with a 'Default' implementation can be written with 'write_default'
            damaged_received_writer.write_default();
        }
    }
}

// Finally these two systems read 'DamageReceived' messages.
//
// The first system will play a sound.
// The second system will spawn a particle effect.
//
// As before, messages are read using an 'MessageReader' by calling 'read'. This returns an iterator over all the &T
// that this system has not read yet.
fn play_damage_received_sound(mut damage_received_reader: MessageReader<DamageReceived>) {
    for _ in damage_received_reader.read() {
        info!("Playing a sound.");
    }
}

// Note that both systems receive the same 'DamageReceived' messages. Any number of systems can
// receive the same message type.
fn play_damage_received_particle_effect(mut damage_received_reader: MessageReader<DamageReceived>) {
    for _ in damage_received_reader.read() {
        info!("Playing particle effect.");
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Messages must be added to the app before they can be used
        // using the 'add_message' method
        .add_message::<DealDamage>()
        .add_message::<ArmorBlockedDamage>()
        .add_message::<DamageReceived>()
        .init_resource::<DamageTimer>()
        // As always we must add our systems to the apps schedule.
        // Here we add our systems to the schedule using 'chain()' so that they run in order
        // This ensures that 'apply_armor_to_damage' runs before 'apply_damage_to_health'
        // It also ensures that 'MessageWriters' are used before the associated 'MessageReaders'
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
        // This means they may have a one frame delay in processing messages compared to the above chain
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
