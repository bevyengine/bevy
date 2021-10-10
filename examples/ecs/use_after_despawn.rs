use bevy::prelude::*;

fn main() {
    App::new()
        .add_startup_system(do_spawn)
        // This system despawns the entity in the Update stage.
        .add_system(do_despawn.system().label("despawn"))
        // This system inserts into the entity in the Update stage.
        // It is scheduled explicitly to run after do_despawn, so its Insert command
        // will panic because in the meantime the Despawn command despawed the entity.
        //
        // Here it is simple. But it could be as complicated a schedule like
        // the inserting system is some system of a third party plugin.
        // About this imagined system you as a user,
        //  you don't immediately know it exists,
        //  you don't know when it is scheduled,
        //  you don't know if it is handling intermittent despawned entities well.
        // But eventually the parallel schedule is run in a way where
        // both systems, your despawner and the third party inserter, run in the same stage
        // in an order where they clash.
        // And when you don't know, which of my entity kinds was it? When was it despawned?
        // How to reproduce it?
        .add_system(do_insert.system().after("despawn"))
        .run();
}

fn do_spawn(mut cmds: Commands) {
    let entity = cmds.spawn().insert(5u32).id();
    println!("{:?} spawn", entity);
}

fn do_despawn(query: Query<Entity, With<u32>>, mut cmds: Commands) {
    for entity in query.iter() {
        println!("{:?} despawn", entity);
        cmds.entity(entity).despawn();
    }
}

fn do_insert(query: Query<Entity, With<u32>>, mut cmds: Commands) {
    for entity in query.iter() {
        println!("{:?} insert", entity);
        cmds.entity(entity).insert("panic".to_string());
    }
}
