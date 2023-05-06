use bevy_reflect::Reflect;

#[derive(Reflect)]
struct Foo<T> {
    a: T,
}

// Type that doesn't implement Reflect
struct NoReflect(f32);

fn main() {
    let mut foo: Box<dyn Reflect> = Box::new(Foo::<NoReflect> { a: NoReflect(42.0) });
    // foo doesn't implement Reflect because NoReflect doesn't implement Reflect
    foo.get_field::<NoReflect>("a").unwrap();
}