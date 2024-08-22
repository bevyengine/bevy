//! Demonstrates how to propagate events through the hierarchy with observers.

use std::time::Duration;

use bevy::{log::LogPlugin, prelude::*, time::common_conditions::on_timer};
use rand::{seq::IteratorRandom, thread_rng, Rng};

fn main() {
    App::new()
        .add_plugins((MinimalPlugins, LogPlugin::default()))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            attack_armor.run_if(on_timer(Duration::from_millis(200))),
        )
        // Add a global observer that will emit a line whenever an attack hits an entity.
        .observe(attack_hits)
        .run();
}

// In this example, we spawn a goblin wearing different pieces of armor. Each piece of armor
// is represented as a child entity, with an `Armor` component.
//
// We're going to model how attack damage can be partially blocked by the goblin's armor using
// event bubbling. Our events will target the armor, and if the armor isn't strong enough to block
// the attack it will continue up and hit the goblin.
fn setup(mut commands: Commands) {
    commands
        .spawn((Name::new("Goblin"), HitPoints(50)))
        .observe(take_damage)
        .with_children(|parent| {
            parent
                .spawn((Name::new("Helmet"), Armor(5)))
                .observe(block_attack);
            parent
                .spawn((Name::new("Socks"), Armor(10)))
                .observe(block_attack);
            parent
                .spawn((Name::new("Shirt"), Armor(15)))
                .observe(block_attack);
        });
}

// This event represents an attack we want to "bubble" up from the armor to the goblin.
#[derive(Clone, Component)]
struct Attack {
    damage: u16,
}

// We enable propagation by implementing `Event` manually (rather than using a derive) and specifying
// two important pieces of information:
impl Event for Attack {
    // 1. Which component we want to propagate along. In this case, we want to "bubble" (meaning propagate
    //    from child to parent) so we use the `Parent` component for propagation. The component supplied
    //    must implement the `Traversal` trait.
    type Traversal = Parent;
    // 2. We can also choose whether or not this event will propagate by default when triggered. If this is
    //    false, it will only propagate following a call to `Trigger::propagate(true)`.
    const AUTO_PROPAGATE: bool = true;
}

/// An entity that can take damage.
#[derive(Component, Deref, DerefMut)]
struct HitPoints(u16);

/// For damage to reach the wearer, it must exceed the armor.
#[derive(Component, Deref)]
struct Armor(u16);

/// A normal bevy system that attacks a piece of the goblin's armor on a timer.
fn attack_armor(entities: Query<Entity, With<Armor>>, mut commands: Commands) {
    let mut rng = thread_rng();
    if let Some(target) = entities.iter().choose(&mut rng) {
        let damage = rng.gen_range(1..20);
        commands.trigger_targets(Attack { damage }, target);
        info!("⚔️  Attack for {} damage", damage);
    }
}

fn attack_hits(trigger: Trigger<Attack>, name: Query<&Name>) {
    if let Ok(name) = name.get(trigger.entity()) {
        info!("Attack hit {}", name);
    }
}

/// A callback placed on [`Armor`], checking if it absorbed all the [`Attack`] damage.
fn block_attack(mut trigger: Trigger<Attack>, armor: Query<(&Armor, &Name)>) {
    let (armor, name) = armor.get(trigger.entity()).unwrap();
    let attack = trigger.event_mut();
    let damage = attack.damage.saturating_sub(**armor);
    if damage > 0 {
        info!("🩸 {} damage passed through {}", damage, name);
        // The attack isn't stopped by the armor. We reduce the damage of the attack, and allow
        // it to continue on to the goblin.
        attack.damage = damage;
    } else {
        info!("🛡️  {} damage blocked by {}", attack.damage, name);
        // Armor stopped the attack, the event stops here.
        trigger.propagate(false);
        info!("(propagation halted early)\n");
    }
}

/// A callback on the armor wearer, triggered when a piece of armor is not able to block an attack,
/// or the wearer is attacked directly.
fn take_damage(
    trigger: Trigger<Attack>,
    mut hp: Query<(&mut HitPoints, &Name)>,
    mut commands: Commands,
    mut app_exit: EventWriter<AppExit>,
) {
    let attack = trigger.event();
    let (mut hp, name) = hp.get_mut(trigger.entity()).unwrap();
    **hp = hp.saturating_sub(attack.damage);

    if **hp > 0 {
        info!("{} has {:.1} HP", name, hp.0);
    } else {
        warn!("💀 {} has died a gruesome death", name);
        commands.entity(trigger.entity()).despawn_recursive();
        app_exit.send(AppExit::Success);
    }

    info!("(propagation reached root)\n");
}
