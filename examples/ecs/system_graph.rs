use bevy::prelude::*;

fn main() {
    // SystemGraphs can be used to specify explicit system dependencies.
    // Labels can be used alongside SystemGraphs to help specify dependency relationships.

    // These three systems will run sequentially one after another.
    let sequential = SystemGraph::new();
    sequential
        .root(print_system("Sequential 1"))
        .then(print_system("Sequential 2"))
        .then(
            print_system("Sequential 3")
                .system()
                .label("Sequential End"),
        );

    // Graphs nodes can fork into multiple dependencies.
    let wide = SystemGraph::new();
    let (mid_1, mid_2, mid_3) = wide
        .root(print_system("Wide Start").system().after("Sequential End"))
        .fork((
            print_system("Wide 1"),
            print_system("Wide 2"),
            print_system("Wide 3"),
        ));

    // Graphs can have multiple root systems.
    let side = wide.root(print_system("Wide Side"));

    // Branches can be continued separately from each other.
    mid_3.then(print_system("Wide 3 Continuation"));

    // Multiple branches can be merged. This system will only run when all dependencies
    // finish running.
    (mid_1, mid_2, side).join(print_system("Wide 1 & Wide 2 Continuation"));

    // SystemGraph implements Into<SystemSet> and can be used to add of the graph's systems to an
    // App.
    App::build()
        .add_system_set(sequential)
        .add_system_set(wide)
        .run();
}

fn print_system(message: &'static str) -> impl Fn() {
    move || {
        println!("{}", message);
    }
}
