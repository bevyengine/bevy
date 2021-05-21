use bevy::prelude::*;

fn main() {
    App::build()
        .add_plugin(bevy::log::LogPlugin::default())
        .add_system(Box::new(|mut cmd: Commands, arg: Local<String>| {
            info!("this system uses an argument: {:?}", arg);
            let id = cmd.spawn().id();
            info!("also it spawned an entity {:?}", id);
        }).system().config(|config| config.1 = Some("hello".to_string())))
        .insert_resource(ExampleResource(123))
        .add_startup_system(create_system(456))
        .add_system(normal_system.system())
        .run();
}

fn normal_system(resource: Res<ExampleResource>) {
    info!("resource has value {:?}", resource.0);
}

pub struct ExampleResource(usize);

// Creates a system that modifies a resource.
fn create_system(arg: usize) -> impl bevy::ecs::system::System<In = (), Out = ()> {
    Box::new(move |mut resource: ResMut<ExampleResource>| {
        info!("system running with captured arg {}", arg);
        resource.0 = arg;
    }).system()
}