use bevy_reflect::{GetField, Reflect, Struct, TypePath};

#[derive(Reflect)]
#[reflect(from_reflect = false)]
struct Foo<T> {
    a: T,
}

// Type that doesn't implement Reflect
#[derive(TypePath)]
struct NoReflect(f32);

fn main() {
    let mut foo: Box<dyn Struct> = Box::new(Foo::<NoReflect> { a: NoReflect(42.0) });
    //~^ ERROR: `NoReflect` does not provide type registration information

    // foo doesn't implement Reflect because NoReflect doesn't implement Reflect
    foo.get_field::<NoReflect>("a").unwrap();
    //~^ ERROR: `NoReflect` can not be reflected
}
