use bevy_reflect::{Reflect, TypePath};

#[derive(Reflect)]
#[reflect(from_reflect = false)]
struct Foo<T> {
    a: T,
}

// Type that doesn't implement Reflect
#[derive(TypePath)]
struct NoReflect(f32);

fn main() {
    let mut foo: Box<dyn Reflect> = Box::new(Foo::<NoReflect> { a: NoReflect(42.0) });
    // foo doesn't implement Reflect because NoReflect doesn't implement Reflect
    foo.get_field::<NoReflect>("a").unwrap();
}
