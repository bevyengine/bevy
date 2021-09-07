use bevy::{ecs::schedule::ReportExecutionOrderAmbiguities, log::LogPlugin, prelude::*};

fn main() {
    App::new()
        .add_plugin(LogPlugin)
        .insert_resource(MyStartupResource(0))
        // This resource allows to control how Ambiguity Checker will report unresolved ambiguities.
        // By default only a warning with the amount of unresolved ambiguities is shown, but
        // a more complete report will be displayed if we explicitly set this resource to verbose.
        .insert_resource(ReportExecutionOrderAmbiguities::verbose())
        // `startup_system_a` and `startup_system_b` will both compete for the same resource. Since there is no ordering between
        // them (e.g., `.before()` or `.after()`), which one will run first is not deterministic.
        // This ambiguity will be reported by ambiguity checker.
        .add_startup_system(startup_system_a)
        .add_startup_system(startup_system_b)
        .insert_resource(MyResource(0))
        .insert_resource(MyAnotherResource(0))
        // It is possible to mark a system as ambiguous if this is intended behavior; the ambiguity checker will ignore this system.
        .add_system(system_a.silence_ambiguity_checks())
        .add_system(system_b.label("my_label"))
        // It is also possible to mark a system as ambiguous with a specific other system,
        // making the checker ignore any ambiguities between them.
        .add_system(system_c.ambiguous_with("my_label"))
        // If there's an whole group of systems that are supposed to be ambiguous with each other,
        // an ambiguity set can be used to make the checker ignore anything it detects between them.
        .add_system(system_d.in_ambiguity_set("my_set"))
        .add_system(system_e.in_ambiguity_set("my_set"))
        .run();
}

struct MyStartupResource(i32);
fn startup_system_a(mut res: ResMut<MyStartupResource>) {
    res.0 += 1;
}

fn startup_system_b(mut res: ResMut<MyStartupResource>) {
    res.0 += 1;
}

struct MyResource(i32);
struct MyAnotherResource(i32);

fn system_a(mut res: ResMut<MyResource>) {
    res.0 += 1;
}

fn system_b(mut res: ResMut<MyResource>, mut another_res: ResMut<MyAnotherResource>) {
    res.0 += 1;
    another_res.0 += 1;
}

fn system_c(mut res: ResMut<MyResource>) {
    res.0 += 1;
}

fn system_d(mut res: ResMut<MyResource>, mut another_res: ResMut<MyAnotherResource>) {
    res.0 += 1;
    another_res.0 += 1;
}

fn system_e(mut res: ResMut<MyResource>) {
    res.0 += 1;
}
