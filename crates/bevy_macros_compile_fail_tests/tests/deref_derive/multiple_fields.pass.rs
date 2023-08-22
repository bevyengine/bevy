use bevy_derive::Deref;

#[derive(Deref)]
struct TupleStruct(usize, #[deref] String);

#[derive(Deref)]
struct Struct {
    foo: usize,
    #[deref]
    bar: String,
}

fn main() {
    let value = TupleStruct(123, "Hello world!".to_string());
    let _: &String = &*value;

    let value = Struct {
        foo: 123,
        bar: "Hello world!".to_string(),
    };
    let _: &String = &*value;
}
