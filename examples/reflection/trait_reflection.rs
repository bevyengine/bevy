//! Allows reflection with trait objects.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .register_type::<MyType>()
        .add_systems(Startup, setup)
        .run();
}

#[derive(Reflect)]
#[reflect(DoThing)]
struct MyType {
    value: String,
}

impl DoThing for MyType {
    fn do_thing(&self) -> String {
        format!("{} World!", self.value)
    }
}

#[reflect_trait]
pub trait DoThing {
    fn do_thing(&self) -> String;
}

fn setup(type_registry: Res<AppTypeRegistry>) {
    // First, lets box our type as a Box<dyn Reflect>
    let reflect_value: Box<dyn Reflect> = Box::new(MyType {
        value: "Hello".to_string(),
    });

    // This means we no longer have direct access to MyType or its methods. We can only call Reflect
    // methods on reflect_value. What if we want to call `do_thing` on our type? We could
    // downcast using reflect_value.downcast_ref::<MyType>(), but what if we don't know the type
    // at compile time?

    // Normally in rust we would be out of luck at this point. Lets use our new reflection powers to
    // do something cool!
    let type_registry = type_registry.read();

    // The #[reflect] attribute we put on our DoThing trait generated a new `ReflectDoThing` struct,
    // which implements TypeData. This was added to MyType's TypeRegistration.
    let reflect_do_thing = type_registry
        .get_type_data::<ReflectDoThing>(reflect_value.type_id())
        .unwrap();

    // We can use this generated type to convert our `&dyn Reflect` reference to a `&dyn DoThing`
    // reference
    let my_trait: &dyn DoThing = reflect_do_thing.get(&*reflect_value).unwrap();

    // Which means we can now call do_thing(). Magic!
    info!("{}", my_trait.do_thing());

    // This works because the #[reflect(MyTrait)] we put on MyType informed the Reflect derive to
    // insert a new instance of ReflectDoThing into MyType's registration. The instance knows
    // how to cast &dyn Reflect to &dyn MyType, because it knows that &dyn Reflect should first
    // be downcasted to &MyType, which can then be safely casted to &dyn MyType
}
