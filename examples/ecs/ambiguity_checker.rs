use bevy::{log::LogPlugin, prelude::*};

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        // This resource allows to control how Ambiguity Checker will report unresolved ambiguities.
        // By default only a warning with the number of unresolved ambiguities is shown, but
        // a more complete report will be displayed if we explicitly set this resource to verbose.
        .insert_resource(ReportExecutionOrderAmbiguities::Verbose)
        .add_plugin(LogPlugin)
        .insert_resource(MyStartupResource(0))
        // `startup_system_a` and `startup_system_b` will both compete for the same resource. Since there is no ordering between
        // them (e.g., `.before()` or `.after()`), which one will run first is not deterministic.
        // This ambiguity will be reported by ambiguity checker.
        .add_startup_system(startup_system_a)
        .add_startup_system(startup_system_b)
        .insert_resource(MyResource(0))
        .insert_resource(MyOtherResource(0))
        // It is possible to mark a system as ambiguous if this is intended behavior; the ambiguity checker will ignore this system.
        .add_system(system_a.ignore_all_ambiguities())
        .add_system(system_b)
        // It is also possible to mark a system as deliberately ambiguous with a provided system or label,
        // making the checker ignore any ambiguities between them.
        .add_system(system_c.ambiguous_with(system_b))
        // If there's an whole group of systems that are supposed to be ambiguous with each other,
        // add a shared label, and then ignore any conflicts with that label.
        .add_system(system_d.label(AmbiguitySet).ambiguous_with(AmbiguitySet))
        .add_system(system_e.label(AmbiguitySet).ambiguous_with(AmbiguitySet))
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
