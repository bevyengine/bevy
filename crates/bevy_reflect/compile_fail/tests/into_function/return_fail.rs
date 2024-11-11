#![allow(unused)]

use bevy_reflect::func::IntoFunction;
use bevy_reflect::Reflect;

fn pass() -> i32 {
    123
}

struct Foo;

fn return_not_reflect() -> Foo {
    Foo
}

fn return_with_lifetime_pass<'a>(a: &'a String) -> &'a String {
    a
}

fn return_with_invalid_lifetime<'a, 'b>(a: &'a String, b: &'b String) -> &'b String {
    b
}

fn main() {
    let _ = pass.into_function();

    let _ = return_not_reflect.into_function();
    //~^ E0599

    let _ = return_with_lifetime_pass.into_function();

    let _ = return_with_invalid_lifetime.into_function();
    //~^ E0599
}
