use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // This resource controls how an app handles pairs of systems that have incompatible access but ambiguous execution order.
        // The default behavior logs a warning with just the number of ambiguous pairs. Setting the resource to "verbose" 
        // will report these pairs in more detail.
        .insert_resource(ExecutionOrderAmbiguities::WarnVerbose)
        .insert_resource(MyStartupResource(0))
        // `startup_system_a` and `startup_system_b` will both compete for the same resource. Since there is no ordering between
        // them (e.g., `.before()` or `.after()`), which one will run first is not deterministic.
        // This ambiguity will be reported by ambiguity checker.
        .add_startup_system(startup_system_a)
        .add_startup_system(startup_system_b)
        .insert_resource(MyResource(0))
        .insert_resource(MyOtherResource(0))
        .add_system(system_a)
        // It's possible to tell the ambiguity checker to ignore conflicts between a specific pair of systems.
        .add_system(system_b.ambiguous_with(system_a))
        // Likewise, between a system and other systems that have a certain label.
        .add_system(system_c.label(AmbiguitySet).ambiguous_with(AmbiguitySet))
        .add_system(system_d.label(AmbiguitySet).ambiguous_with(AmbiguitySet))
        // Lastly, if desired, the checker can be told to ignore any conflicts that involve a particular system.
        .add_system(system_e.ignore_all_ambiguities())
        .run();
}

#[derive(SystemLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct AmbiguitySet;

struct MyStartupResource(i32);
fn startup_system_a(mut res: ResMut<MyStartupResource>) {
    res.0 += 1;
}

fn startup_system_b(mut res: ResMut<MyStartupResource>) {
    res.0 += 1;
}

struct MyResource(i32);
struct MyOtherResource(i32);

fn system_a(mut res: ResMut<MyResource>) {
    res.0 += 1;
}

fn system_b(mut res: ResMut<MyResource>, mut another_res: ResMut<MyOtherResource>) {
    res.0 += 1;
    another_res.0 += 1;
}

fn system_c(mut res: ResMut<MyResource>) {
    res.0 += 1;
}

fn system_d(mut res: ResMut<MyResource>, mut another_res: ResMut<MyOtherResource>) {
    res.0 += 1;
    another_res.0 += 1;
}

fn system_e(mut res: ResMut<MyResource>) {
    res.0 += 1;
}
