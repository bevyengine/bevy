use bevy::{
    app::{AppExit, ScheduleRunnerPlugin, ScheduleRunnerSettings},
    ecs::schedule::ReportExecutionOrderAmbiguities,
    log::LogPlugin,
    prelude::*,
    utils::Duration,
};
use rand::random;

/// This is a guided introduction to Bevy's "Entity Component System" (ECS)
/// All Bevy app logic is built using the ECS pattern, so definitely pay attention!
///
/// Why ECS?
/// * Data oriented: Functionality is driven by data
/// * Clean Architecture: Loose coupling of functionality / prevents deeply nested inheritance
/// * High Performance: Massively parallel and cache friendly
///
/// ECS Definitions:
///
/// Component: just a normal Rust data type. generally scoped to a single piece of functionality
///     Examples: position, velocity, health, color, name
///
/// Entity: a collection of components with a unique id
///     Examples: Entity1 { Name("Alice"), Position(0, 0) }, Entity2 { Name("Bill"), Position(10, 5)
/// }

/// Resource: a shared global piece of data
///     Examples: asset_storage, events, system state
///
/// System: runs logic on entities, components, and resources
///     Examples: move_system, damage_system
///
/// Now that you know a little bit about ECS, lets look at some Bevy code!
/// We will now make a simple "game" to illustrate what Bevy's ECS looks like in practice.

// COMPONENTS: Pieces of functionality we add to entities. These are just normal Rust data types
//

// Our game will have a number of "players". Each player has a name that identifies them
#[derive(Component)]
struct Player {
    name: String,
}

// Each player also has a score. This component holds on to that score
#[derive(Component)]
struct Score {
    value: usize,
}

// RESOURCES: "Global" state accessible by systems. These are also just normal Rust data types!
//

// This resource holds information about the game:
#[derive(Default)]
struct GameState {
    current_round: usize,
    total_players: usize,
    winning_player: Option<String>,
}

// This resource provides rules for our "game".
struct GameRules {
    winning_score: usize,
    max_rounds: usize,
    max_players: usize,
}

// SYSTEMS: Logic that runs on entities, components, and resources. These generally run once each
// time the app updates.
//

// This is the simplest type of system. It just prints "This game is fun!" on each run:
fn print_message_system() {
    println!("This game is fun!");
}

// Systems can also read and modify resources. This system starts a new "round" on each update:
// NOTE: "mut" denotes that the resource is "mutable"
// Res<GameRules> is read-only. ResMut<GameState> can modify the resource
fn new_round_system(game_rules: Res<GameRules>, mut game_state: ResMut<GameState>) {
    game_state.current_round += 1;
    println!(
        "Begin round {} of {}",
        game_state.current_round, game_rules.max_rounds
    );
}

// This system updates the score for each entity with the "Player" and "Score" component.
fn score_system(mut query: Query<(&Player, &mut Score)>) {
    for (player, mut score) in query.iter_mut() {
        let scored_a_point = random::<bool>();
        if scored_a_point {
            score.value += 1;
            println!(
                "{} scored a point! Their score is: {}",
                player.name, score.value
            );
        } else {
            println!(
                "{} did not score a point! Their score is: {}",
                player.name, score.value
            );
        }
    }

    // this game isn't very fun is it :)
}

// This system runs on all entities with the "Player" and "Score" components, but it also
// accesses the "GameRules" resource to determine if a player has won.
fn score_check_system(
    game_rules: Res<GameRules>,
    mut game_state: ResMut<GameState>,
    query: Query<(&Player, &Score)>,
) {
    for (player, score) in query.iter() {
        if score.value == game_rules.winning_score {
            game_state.winning_player = Some(player.name.clone());
        }
    }
}

// This system ends the game if we meet the right conditions. This fires an AppExit event, which
// tells our App to quit. Check out the "event.rs" example if you want to learn more about using
// events.
fn game_over_system(
    game_rules: Res<GameRules>,
    game_state: Res<GameState>,
    mut app_exit_events: EventWriter<AppExit>,
) {
    if let Some(ref player) = game_state.winning_player {
        println!("{} won the game!", player);
        app_exit_events.send(AppExit);
    } else if game_state.current_round == game_rules.max_rounds {
        println!("Ran out of rounds. Nobody wins!");
        app_exit_events.send(AppExit);
    }

    println!();
}

// This is a "startup" system that runs exactly once when the app starts up. Startup systems are
// generally used to create the initial "state" of our game. The only thing that distinguishes a
// "startup" system from a "normal" system is how it is registered:      Startup:
// app.add_startup_system(startup_system)      Normal:  app.add_system(normal_system)
fn startup_system(mut commands: Commands, mut game_state: ResMut<GameState>) {
    // Create our game rules resource
    commands.insert_resource(GameRules {
        max_rounds: 10,
        winning_score: 4,
        max_players: 4,
    });

    // Add some players to our world. Players start with a score of 0 ... we want our game to be
    // fair!
    commands.spawn_batch(vec![
        (
            Player {
                name: "Alice".to_string(),
            },
            Score { value: 0 },
        ),
        (
            Player {
                name: "Bob".to_string(),
            },
            Score { value: 0 },
        ),
    ]);

    // set the total players to "2"
    game_state.total_players = 2;
}

// This system uses a command buffer to (potentially) add a new player to our game on each
// iteration. Normal systems cannot safely access the World instance directly because they run in
// parallel. Our World contains all of our components, so mutating arbitrary parts of it in parallel
// is not thread safe. Command buffers give us the ability to queue up changes to our World without
// directly accessing it
fn new_player_system(
    mut commands: Commands,
    game_rules: Res<GameRules>,
    mut game_state: ResMut<GameState>,
) {
    // Randomly add a new player
    let add_new_player = random::<bool>();
    if add_new_player && game_state.total_players < game_rules.max_players {
        game_state.total_players += 1;
        commands.spawn_bundle((
            Player {
                name: format!("Player {}", game_state.total_players),
            },
            Score { value: 0 },
        ));

        println!("Player {} joined the game!", game_state.total_players);
    }
}

// If you really need full, immediate read/write access to the world or resources, you can use a
// "thread local system". These run on the main app thread (hence the name "thread local")
// WARNING: These will block all parallel execution of other systems until they finish, so they
// should generally be avoided if you care about performance
#[allow(dead_code)]
fn thread_local_system(world: &mut World) {
    // this does the same thing as "new_player_system"
    let total_players = world.get_resource_mut::<GameState>().unwrap().total_players;
    let should_add_player = {
        let game_rules = world.get_resource::<GameRules>().unwrap();
        let add_new_player = random::<bool>();
        add_new_player && total_players < game_rules.max_players
    };
    // Randomly add a new player
    if should_add_player {
        world.spawn().insert_bundle((
            Player {
                name: format!("Player {}", total_players),
            },
            Score { value: 0 },
        ));

        let mut game_state = world.get_resource_mut::<GameState>().unwrap();
        game_state.total_players += 1;
    }
}

// Sometimes systems need their own unique "local" state. Bevy's ECS provides Local<T> resources for
// this case. Local<T> resources are unique to their system and are automatically initialized on
// your behalf (if they don't already exist). If you have a system's id, you can also access local
// resources directly in the Resources collection using `Resources::get_local()`. In general you
// should only need this feature in the following cases:  1. You have multiple instances of the same
// system and they each need their own unique state  2. You already have a global version of a
// resource that you don't want to overwrite for your current system  3. You are too lazy to
// register the system's resource as a global resource

#[derive(Default)]
struct State {
    counter: usize,
}

// NOTE: this doesn't do anything relevant to our game, it is just here for illustrative purposes
#[allow(dead_code)]
fn local_state_system(mut state: Local<State>, query: Query<(&Player, &Score)>) {
    for (player, score) in query.iter() {
        println!("processed: {} {}", player.name, score.value);
    }
    println!("this system ran {} times", state.counter);
    state.counter += 1;
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
enum MyStage {
    BeforeRound,
    AfterRound,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
enum MyLabels {
    ScoreCheck,
}

// Our Bevy app's entry point
fn main() {
    // Bevy apps are created using the builder pattern. We use the builder to add systems,
    // resources, and plugins to our app
    App::new()
        // Resources can be added to our app like this
        .insert_resource(State { counter: 0 })
        // Some systems are configured by adding their settings as a resource
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs(5)))
        // Plugins are just a grouped set of app builder calls (just like we're doing here).
        // We could easily turn our game into a plugin, but you can check out the plugin example for
        // that :) The plugin below runs our app's "system schedule" once every 5 seconds
        // (configured above).
        .add_plugin(ScheduleRunnerPlugin::default())
        // Resources that implement the Default or FromWorld trait can be added like this:
        .init_resource::<GameState>()
        // Startup systems run exactly once BEFORE all other systems. These are generally used for
        // app initialization code (ex: adding entities and resources)
        .add_startup_system(startup_system)
        // my_system calls converts normal rust functions into ECS systems:
        .add_system(print_message_system)
        // SYSTEM EXECUTION ORDER
        //
        // Each system belongs to a `Stage`, which controls the execution strategy and broad order
        // of the systems within each tick. Startup stages (which startup systems are
        // registered in) will always complete before ordinary stages begin,
        // and every system in a stage must complete before the next stage advances.
        // Once every stage has concluded, the main loop is complete and begins again.
        //
        // By default, all systems run in parallel, except when they require mutable access to a
        // piece of data. This is efficient, but sometimes order matters.
        // For example, we want our "game over" system to execute after all other systems to ensure
        // we don't accidentally run the game for an extra round.
        //
        // Rather than splitting each of your systems into separate stages, you should force an
        // explicit ordering between them by giving the relevant systems a label with
        // `.label`, then using the `.before` or `.after` methods. Systems will not be
        // scheduled until all of the systems that they have an "ordering dependency" on have
        // completed.
        //
        // Doing that will, in just about all cases, lead to better performance compared to
        // splitting systems between stages, because it gives the scheduling algorithm more
        // opportunities to run systems in parallel.
        // Stages are still necessary, however: end of a stage is a hard sync point
        // (meaning, no systems are running) where `Commands` issued by systems are processed.
        // This is required because commands can perform operations that are incompatible with
        // having systems in flight, such as spawning or deleting entities,
        // adding or removing resources, etc.
        //
        // add_system(system) adds systems to the UPDATE stage by default
        // However we can manually specify the stage if we want to. The following is equivalent to
        // add_system(score_system)
        .add_system_to_stage(CoreStage::Update, score_system)
        // We can also create new stages. Here is what our games stage order will look like:
        // "before_round": new_player_system, new_round_system
        // "update": print_message_system, score_system
        // "after_round": score_check_system, game_over_system
        .add_stage_before(
            CoreStage::Update,
            MyStage::BeforeRound,
            SystemStage::parallel(),
        )
        .add_stage_after(
            CoreStage::Update,
            MyStage::AfterRound,
            SystemStage::parallel(),
        )
        .add_system_to_stage(MyStage::BeforeRound, new_round_system)
        .add_system_to_stage(MyStage::BeforeRound, new_player_system)
        // We can ensure that game_over system runs after score_check_system using explicit ordering
        // constraints First, we label the system we want to refer to using `.label`
        // Then, we use either `.before` or `.after` to describe the order we want the relationship
        .add_system_to_stage(
            MyStage::AfterRound,
            score_check_system.label(MyLabels::ScoreCheck),
        )
        .add_system_to_stage(
            MyStage::AfterRound,
            game_over_system.after(MyLabels::ScoreCheck),
        )
        // We can check our systems for execution order ambiguities by examining the output produced
        // in the console by using the `LogPlugin` and adding the following Resource to our App :)
        // Be aware that not everything reported by this checker is a potential problem, you'll have
        // to make that judgement yourself.
        .add_plugin(LogPlugin::default())
        .insert_resource(ReportExecutionOrderAmbiguities)
        // This call to run() starts the app we just built!
        .run();
}
