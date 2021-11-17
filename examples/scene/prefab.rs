use bevy::{ecs::schedule::ShouldRun, prelude::*, reflect::TypeRegistry, scene::SceneSpawnError, utils::HashMap};
/// Basic Prefab support for Bevy 0.5.
/// To use the factory, spawn a component using Commands or World, and the Handle<DynamicScene> of the prefab.
/// Then add to the factory queue. When the scene is loaded, 
/// the components of the first entity listed in the file will be added to the entity.

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut prefab_factory: ResMut<PrefabFactory>,
) {
    let handle: Handle<DynamicScene> = asset_server.load("my_scene/path.scn.ron");

    let prefab_entity = commands.spawn().insert(TrackingComponent).id();

    prefab_factory.add_to_queue(prefab_entity, handle);
}

pub struct TrackingComponent;

#[derive(Reflect, Clone)]
#[reflect(Component)]
struct A {
    message: String,
}

impl Default for A {
    fn default() -> Self {
        A { message: "Hello!".to_string() }
    }
}
#[derive(Reflect, Clone, Default)]
#[reflect(Component)]
struct B {
    data: Vec<usize>,
}

fn locate_prefab(
    query: Query<(&A, &B), (Added<A>, With<TrackingComponent>)>,
) {
    for (comp_a, comp_b) in query.iter() {
        println!("{}, {:?}", comp_a.message, comp_b.data);
    }
}

fn main() {
    let mut app = App::build();
    app
        .init_resource::<PrefabFactory>()
        .register_type::<A>
        .register_type::<B>
        .add_system_to_stage(
            CoreStage::Last, 
            prefab_factory_system_ex.exclusive_system()
                .with_run_criteria(run_if_queue_occupied.system())
        )
        .add_startup_system(setup.system()).label("setup")
        .add_startup_system(write_scene.exclusive_system()).before("setup")
    ;

    app.run()
}



#[derive(Default)]
pub struct PrefabFactory {
    queue: HashMap<Entity, Handle<DynamicScene>>,
}

impl PrefabFactory {
    pub fn add_to_queue(&mut self, entity: Entity, handle: Handle<DynamicScene>) {
        self.queue.insert(entity, handle);
    }
}
// Run condition to prevent the factory from running while empty.
pub fn run_if_queue_occupied(
    prefab_factory: Res<PrefabFactory>
) -> ShouldRun {
    match !prefab_factory.queue.is_empty() {
        false => ShouldRun::No,
        true => ShouldRun::Yes,
    }
}

pub fn prefab_factory_system_ex(world: &mut World) {
    world.resource_scope(|world, mut factory: Mut<PrefabFactory>| {
        let registry = world.get_resource::<TypeRegistry>().unwrap().clone();
        let type_registry = registry.read();
        let mut complete = Vec::default();

        world.resource_scope(|world, dynamic_scenes: Mut<Assets<DynamicScene>>| {
            for (entity, handle) in factory.queue.iter() {
                if let Some(scene) = dynamic_scenes.get(handle) {
                    for component in scene.entities.first().unwrap().components.iter() {
                        let registration = type_registry
                            .get_with_name(component.type_name())
                            .ok_or_else(|| SceneSpawnError::UnregisteredType {
                                type_name: component.type_name().to_string(),
                            }).unwrap();
                        let reflect_component =
                            registration.data::<ReflectComponent>().ok_or_else(|| {
                                SceneSpawnError::UnregisteredComponent {
                                    type_name: component.type_name().to_string(),
                                }
                            }).unwrap();
                        if world
                            .entity(*entity)
                            .contains_type_id(registration.type_id())
                        {
                            reflect_component.apply_component(world, *entity, &**component);
                        } else {
                            reflect_component.add_component(world, *entity, &**component);
                        }
                    }
                    complete.push(*entity);
                }
            };
        });
        
        for entity in complete {
            factory.queue.remove(&entity);
        }

    })
}

fn write_scene(&world: &mut World) {
    let mut scene_world = World::new();
    
    let path = "assets/my_scene/path.scn.ron";

    scene_world
        .spawn()
        .insert(A {"My scene prefab is here!".to_string()})
        .insert(B {
            data: vec![
                5, 7, 8, 22, 42, 1001
            ]
        })
    ;
    
    let type_registry = world.get_resource::<TypeRegistry>().unwrap();
    let scene = DynamicScene::from_world(&scene_world, type_registry);

    let mut file = match File::create(path) {
        Err(why) => panic!("Failed to create file: {}", why),
        Ok(file) => file,
    };

    match file.write_all(scene.serialize_ron(&type_registry).unwrap().as_bytes()) {
        Err(why) => panic!("couldn't write to file: {}", why),
        Ok(_) => println!("File write success"),
    }
}