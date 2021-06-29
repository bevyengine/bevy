use bevy::{core::FixedTimestep, prelude::*};

fn main() {
    App::build()
        .insert_resource(FailedDespawnAttempts(0))
        .add_startup_system(setup.system())
        .add_system(
            remove_components
                .system()
                .with_run_criteria(FixedTimestep::step(0.5)),
        )
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(2.0))
                .with_system(despawn_all_entities.system())
                .with_system(log_failed_despawn_attempts.system()),
        )
        .run();
}

struct A(usize);

#[derive(Bundle, Default)]
struct B {
    value: usize,
}

struct FailedDespawnAttempts(usize);

fn setup(mut commands: Commands) {
    for i in 0..3 {
        // Note that `insert` and `insert_bundle` are fallible functions.
        // If no error handler is specified, the default behavior is to log the error, and continue.
        // However, these calls to `insert` and `insert_bundle` will not fail, since the entity is valid.
        commands.spawn().insert(A(i)).insert_bundle(B::default());
    }
}

fn log_failed_despawn_attempts(attempts: Res<FailedDespawnAttempts>) {
    info!("There have been {} failed despawn attempts!", attempts.0);
}

fn despawn_all_entities(mut commands: Commands, query: Query<Entity>) {
    for e in query.iter() {
        // `on_err` also allows you to provide a custom error handler!
        commands.entity(e).despawn().on_err(|error, ctx| {
            // You'll notice that the `error` will also give you back the entity
            // you tried to despawn.
            let entity = error.entity;

            warn!("Sadly our entity '{:?}' didn't despawn :(", entity);

            // error handlers have mutable access to `World`
            if let Some(mut failed_despawns) = ctx.world.get_resource_mut::<FailedDespawnAttempts>()
            {
                failed_despawns.0 += 1;
            }
        });
    }
}

fn remove_components(mut commands: Commands, query: Query<Entity>) {
    for e in query.iter() {
        // Some nice things:
        // - You can still chain commands!
        // - There are a slew of built-in error handlers
        commands
            .entity(e)
            .remove::<A>()
            // `CommandErrorHandler::ignore` will neither log nor panic the error
            .on_err(CommandErrorHandler::ignore)
            .remove_bundle::<B>()
            // `CommandErrorHandler::log` is the default behavior, and will log the error.
            // `CommandErrorHandler::panic` is another alternative which will panic on the error.
            .on_err(CommandErrorHandler::log);
    }
}
