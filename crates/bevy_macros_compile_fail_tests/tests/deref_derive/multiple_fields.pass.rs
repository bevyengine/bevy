use bevy_derive::Deref;

#[derive(Deref)]
struct TupleStruct(usize, #[deref] String);

#[derive(Deref)]
struct Struct {
    // Works with other attributes
    #[cfg(test)]
    foo: usize,
    #[deref]
    bar: String,
    /// Also works with doc comments.
    baz: i32,
}

fn main() {
    let value = TupleStruct(123, "Hello world!".to_string());
    let _: &String = &*value;

    let value = Struct {
        #[cfg(test)]
        foo: 123,
        bar: "Hello world!".to_string(),
        baz: 321,
    };
    let _: &String = &*value;
}
