use bevy_derive::Deref;

#[derive(Deref)]
struct TupleStruct(String);

#[derive(Deref)]
struct Struct {
    bar: String,
}

fn main() {
    let value = TupleStruct("Hello world!".to_string());
    let _: &String = &*value;

    let value = Struct {
        bar: "Hello world!".to_string(),
    };
    let _: &String = &*value;
}
