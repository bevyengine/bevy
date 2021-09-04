use bevy::{ecs::schedule::ReportExecutionOrderAmbiguities, log::LogPlugin, prelude::*};

fn main() {
    App::new()
        .add_plugin(LogPlugin)
        .insert_resource(MyStartupResource(0))
        .insert_resource(1i32)
        // This resource allows to control how Ambiguity Checker will report unresolved ambiguities
        // By default only a warning numbering the unresolved ambiguities count is show, but by
        // explicitly setting it to verbose, a complete report is shown
        .insert_resource(ReportExecutionOrderAmbiguities::verbose())
        // startup_system_a and startup_system_b will both compete by the same resource. Since there is no ordering between
        // both of them (like using .before or .after) there is no guarantee which one will take resource first.
        // This ambiguity will be reported by Ambiguity Checker.
        .add_startup_system(startup_system_a)
        .add_startup_system(startup_system_b)
        .insert_resource(MyResource(0))
        // It is possible to mark a system as ambiguous if this is a intended behavior, so the ambiguity checker will ignore this system.
        .add_system(system_a.ambiguous())
        .add_system(system_b.label("my_label"))
        // It is also possible to mark a system as ambiguous with a given label, so whenever ambiguity checker find a ambiguity between
        // this system and anyone with the given label, it will ignore, since this is an intended behavior.
        .add_system(system_c.ambiguous_with("my_label"))
        // If a given set of systems all are ambiguous with each other and this is fine, one may create an ambiguity set, so all systems
        // inside this ambiguity set will be ignored by ambiguity checker
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
fn system_a(mut res: ResMut<MyResource>) {
    res.0 += 1;
}

fn system_b(mut res: ResMut<MyResource>, mut another_res: ResMut<i32>) {
    res.0 += 1;
    *another_res += 1;
}

fn system_c(mut res: ResMut<MyResource>) {
    res.0 += 1;
}

fn system_d(mut res: ResMut<MyResource>, mut another_res: ResMut<i32>) {
    res.0 += 1;
    *another_res += 1;
}

fn system_e(mut res: ResMut<MyResource>) {
    res.0 += 1;
}
