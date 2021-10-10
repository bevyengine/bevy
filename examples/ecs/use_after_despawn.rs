use bevy::prelude::*;

fn main() {
    App::new()
        .add_startup_system(do_spawn)
        .add_system(do_despawn.system().label("despawn"))
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
