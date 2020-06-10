use bevy::{app::AppExit, prelude::*};
use rand::random;
use std::time::Duration;

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
///     Examples: Entity1 { Name("Alice"), Position(0, 0) }, Entity2 { Name("Bill"), Position(10, 5) }

/// Resource: a shared global piece of data
///     Examples: asset_storage, events, system state
///
/// System: runs logic on entities, components, and resources
///     Examples: move_system, damage_system
///
/// Now that you know a little bit about ECS, lets look at some Bevy code!
/// We will now make a simple "game" to illustrate what Bevy's ECS looks like in practice.

//
// COMPONENTS: Pieces of functionality we add to entities. These are just normal Rust data types
//

// Our game will have a number of "players". Each player has a name that identifies them
struct Player {
    name: String,
}

// Each player also has a score. This component holds on to that score
struct Score {
    value: usize,
}

//
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

//
// SYSTEMS: Logic that runs on entities, components, and resources. These generally run once each time the app updates.
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

// This system runs once for each entity with both the "Player" and "Score" component.
// NOTE: Com<Player> is a read-only component reference, whereas ComMut<Score> can modify the component
fn score_system(player: Com<Player>, mut score: ComMut<Score>) {
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

    // this game isn't very fun is it :)
}

// This system runs on all entities with the "Player" and "Score" components, but it also
// accesses the "GameRules" resource to determine if a player has won.
// NOTE: resources must always come before components in system functions
fn score_check_system(
    game_rules: Res<GameRules>,
    mut game_state: ResMut<GameState>,
    player: Com<Player>,
    score: Com<Score>,
) {
    if score.value == game_rules.winning_score {
        game_state.winning_player = Some(player.name.clone());
    }
}

// If you need more control over iteration or direct access to SubWorld, you can also use "query systems"
// This is how you would represent the system above with a "query system"
// NOTE: You can add as many queries as you want, but they must come after all resources (Res/ResMut).
#[allow(dead_code)]
fn query_score_check_system(
    world: &mut SubWorld,
    game_rules: Res<GameRules>,
    mut game_state: ResMut<GameState>,
    query: &mut Query<(Read<Player>, Read<Score>)>,
) {
    for (player, score) in query.iter(world) {
        if score.value == game_rules.winning_score {
            game_state.winning_player = Some(player.name.clone());
        }
    }
}

// This system ends the game if we meet the right conditions. This fires an AppExit event, which tells our
// App to quit. Check out the "event.rs" example if you want to learn more about using events.
fn game_over_system(
    game_rules: Res<GameRules>,
    game_state: Res<GameState>,
    mut app_exit_events: ResMut<Events<AppExit>>,
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

// This is a "startup" system that runs exactly once when the app starts up. Startup systems are generally used to create
// the initial "state" of our game. The only thing that distinguishes a "startup" system from a "normal" system is how it is registered:
//      Startup: app.add_startup_system(startup_system)
//      Normal:  app.add_system(normal_system)
// This startup system needs direct access to the ECS World and Resources, which makes it a "thread local system".
// That being said, startup systems can use any of the system forms we've covered. We will cover thread local systems more in a bit.
fn startup_system(world: &mut World, resources: &mut Resources) {
    // Create our game rules resource
    resources.insert(GameRules {
        max_rounds: 10,
        winning_score: 4,
        max_players: 4,
    });

    // Add some players to our world. Players start with a score of 0 ... we want our game to be fair!
    world.insert(
        (),
        vec![
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
        ],
    );

    // set the total players to "2"
    let mut game_state = resources.get_mut::<GameState>().unwrap();
    game_state.total_players = 2;
}

// This system uses a command buffer to (potentially) add a new player to our game on each iteration.
// Normal systems cannot safely access the World instance directly because they run in parallel.
// Our World contains all of our components, so accessing it in parallel is not thread safe.
// Command buffers give us the ability to queue up changes to our World without directly accessing it
// NOTE: Command buffers must always come before resources and components in system functions
fn new_player_system(
    command_buffer: &mut CommandBuffer,
    game_rules: Res<GameRules>,
    mut game_state: ResMut<GameState>,
) {
    // Randomly add a new player
    let add_new_player = random::<bool>();
    if add_new_player && game_state.total_players < game_rules.max_players {
        game_state.total_players += 1;
        command_buffer.insert(
            (),
            vec![(
                Player {
                    name: format!("Player {}", game_state.total_players),
                },
                Score { value: 0 },
            )],
        );

        println!("Player {} joined the game!", game_state.total_players);
    }
}

// If you really need full, immediate read/write access to the world or resources, you can use a "thread local system".
// These run on the main app thread (hence the name "thread local")
// WARNING: These will block all parallel execution of other systems until they finish, so they should generally be avoided if you
// care about performance
// NOTE: You may notice that this function signature looks exactly like the "startup_system" above.
// Thats because they are both thread local!
#[allow(dead_code)]
fn thread_local_system(world: &mut World, resources: &mut Resources) {
    // this does the same thing as "new_player_system"
    let mut game_state = resources.get_mut::<GameState>().unwrap();
    let game_rules = resources.get::<GameRules>().unwrap();
    // Randomly add a new player
    let add_new_player = random::<bool>();
    if add_new_player && game_state.total_players < game_rules.max_players {
        world.insert(
            (),
            vec![(
                Player {
                    name: format!("Player {}", game_state.total_players),
                },
                Score { value: 0 },
            )],
        );

        game_state.total_players += 1;
    }
}

// Closures are like normal systems, but they also "capture" variables, which they can use as local state.
// This system captures the "counter" variable and uses it to maintain a count across executions
// NOTE: This function returns a Box<dyn Schedulable> type. If you are new to rust don't worry! All you
// need to know for now is that the Box contains our system AND the state it captured.
// The .system() call converts the function into the Box<dyn Schedulable> type. We will use the same approach
// when we add our other systems to our app in the main() function below.
#[allow(dead_code)]
fn closure_system() -> Box<dyn Schedulable> {
    let mut counter = 0;
    (move |player: Com<Player>, score: Com<Score>| {
        println!("processed: {} {}", player.name, score.value);
        println!("this ran {} times", counter);
        counter += 1;
    })
    .system()
}

// Closure systems should be avoided in general because they hide state from the ECS. This makes scenarios
// like "saving", "networking/multiplayer", and "replays" much harder.
// Instead you should use the "state" pattern whenever possible:

struct State {
    counter: usize,
}

// NOTE: this doesn't do anything relevant to our game, it is just here for illustrative purposes
#[allow(dead_code)]
fn stateful_system(mut state: ComMut<State>, player: Com<Player>, score: ComMut<Score>) {
    println!("processed: {} {}", player.name, score.value);
    println!("this ran {} times", state.counter);
    state.counter += 1;
}

// If you need more flexibility, you can define complex systems using "system builders".
// The main features SystemBuilder currently provides over "function style systems" are:
//     * "query filters": filter components in your queries based on some criteria (ex: changed components)
//     * "additional components": Enables access to a component in your SubWorld, even if it isn't in your queries,
// NOTE: this doesn't do anything relevant to our game, it is just here for illustrative purposes
#[allow(dead_code)]
fn complex_system(resources: &mut Resources) -> Box<dyn Schedulable> {
    let game_state = resources.get::<GameState>().unwrap();
    let initial_player_count = game_state.total_players;
    SystemBuilder::new("complex_system")
        .read_resource::<GameState>()
        .write_resource::<GameRules>()
        .read_component::<Draw>()
        // this query is equivalent to the system we saw above: system(player: Com<Player>, mut score: ComMut<Score>)
        .with_query(<(Read<Player>, Write<Score>)>::query())
        // this query only returns entities with a Player component that has changed since the last update
        .with_query(<Read<Player>>::query().filter(changed::<Player>()))
        .build(
            move |_command_buffer,
                  world,
                  (_game_state, _game_rules),
                  (player_score_query, player_changed_query)| {
                println!("The game started with {} players", initial_player_count);

                for (player, score) in player_score_query.iter_mut(world) {
                    println!("processed : {} {}", player.name, score.value);
                }

                for player in player_changed_query.iter(world) {
                    println!("This player was modified: {}", player.name);
                }
            },
        )
}

// Our Bevy app's entry point
fn main() {
    // Bevy apps are created using the builder pattern. We use the builder to add systems, resources, and plugins to our app
    App::build()
        // Plugins are just a grouped set of app builder calls (just like we're doing here).
        // We could easily turn our game into a plugin, but you can check out the plugin example for that :)
        // The plugin below runs our app's "system schedule" once every 5 seconds.
        .add_plugin(ScheduleRunnerPlugin::run_loop(Duration::from_secs(5)))
        // Resources can be added to our app like this
        .add_resource(State { counter: 0 })
        // Resources that implement the Default or FromResources trait can be added like this:
        .init_resource::<GameState>()
        // Startup systems run exactly once BEFORE all other systems. These are generally used for
        // app initialization code (ex: adding entities and resources)
        .add_startup_system(startup_system)
        // my_system.system() calls converts normal rust functions into ECS systems:
        .add_system(print_message_system.system())
        // Systems that need a reference to Resources to be constructed can be added using "init_system":
        // .init_system(complex_system)
        //
        // SYSTEM EXECUTION ORDER
        //
        // By default, all systems run in parallel. This is efficient, but sometimes order matters.
        // For example, we want our "game over" system to execute after all other systems to ensure we don't
        // accidentally run the game for an extra round.
        //
        // First, if a system writes a component or resource (ComMut / ResMut), it will force a synchronization.
        // Any systems that access the data type and were registered BEFORE the system will need to finish first.
        // Any systems that were registered _after_ the system will need to wait for it to finish. This is a great
        // default that makes everything "just work" as fast as possible without us needing to think about it ... provided
        // we don't care about execution order. If we do care, one option would be to use the rules above to force a synchronization
        // at the right time. But that is complicated and error prone!
        //
        // This is where "stages" come in. A "stage" is a group of systems that execute (in parallel). Stages are executed in order,
        // and the next stage won't start until all systems in the current stage have finished.
        // add_system(system) adds systems to the UPDATE stage by default
        // However we can manually specify the stage if we want to. The following is equivalent to add_system(score_system.system())
        .add_system_to_stage(stage::UPDATE, score_system.system())
        // We can also create new stages. Here is what our games stage order will look like:
        // "before_round": new_player_system, new_round_system
        // "update": print_message_system, score_system
        // "after_round": score_check_system, game_over_system
        .add_stage_before(stage::UPDATE, "before_round")
        .add_stage_after(stage::UPDATE, "after_round")
        .add_system_to_stage("before_round", new_round_system.system())
        .add_system_to_stage("before_round", new_player_system.system())
        .add_system_to_stage("after_round", score_check_system.system())
        .add_system_to_stage("after_round", game_over_system.system())
        // score_check_system will run before game_over_system because score_check_system modifies GameState and game_over_system
        // reads GameState. This works, but it's a bit confusing. In practice, it would be clearer to create a new stage that runs
        // before "after_round"
        // This call to run() starts the app we just built!
        .run();
}
