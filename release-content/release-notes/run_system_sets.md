---
title: Run individual System Sets in Schedules
authors: ["@ItsDoot"]
pull_requests: [21893]
---

You can now run a specific system set within a schedule without executing the entire
schedule! This is particularly useful for testing, debugging, or selectively running
parts of your game logic, all without needing to factor features out into separate
schedules:

```rust
use bevy::prelude::*;

#[derive(SystemSet, Clone, Copy, PartialEq, Eq, Debug, Hash)]
enum GameSystems {
    Physics,
    Combat,
    UI,
}

fn physics_system() { /* ... */ }
fn combat_system() { /* ... */ }
fn ui_system() { /* ... */ }

let mut schedule = Schedule::default();
schedule.add_systems((
    physics_system.in_set(GameSystems::Physics),
    combat_system.in_set(GameSystems::Combat),
    ui_system.in_set(GameSystems::UI),
));

let mut world = World::new();

// Run only the physics systems
schedule.run_system_set(&mut world, GameSystems::Physics);

// Run only the combat systems
schedule.run_system_set(&mut world, GameSystems::Combat);

// You can also run system sets from the World or via Commands:
world.run_system_set(MySchedule, MySet);
commands.run_system_set(MySchedule, MySet);
```
