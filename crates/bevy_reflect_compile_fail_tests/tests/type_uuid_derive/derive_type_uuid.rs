use bevy_reflect::TypeUuid;

fn main() {}

// Missing #[uuid] attribute
#[derive(TypeUuid)]
struct A;

// Malformed attribute
#[derive(TypeUuid)]
#[uuid = 42]
struct B;

// UUID parse fail
#[derive(TypeUuid)]
#[uuid = "000"]
struct C;
