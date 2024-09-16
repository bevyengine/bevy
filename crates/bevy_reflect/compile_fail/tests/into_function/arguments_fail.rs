#![allow(unused)]

use bevy_reflect::func::IntoFunction;
use bevy_reflect::Reflect;

fn pass(_: i32) {}

fn too_many_arguments(
    arg0: i32,
    arg1: i32,
    arg2: i32,
    arg3: i32,
    arg4: i32,
    arg5: i32,
    arg6: i32,
    arg7: i32,
    arg8: i32,
    arg9: i32,
    arg10: i32,
    arg11: i32,
    arg12: i32,
    arg13: i32,
    arg14: i32,
    arg15: i32,
) {
}

struct Foo;

fn argument_not_reflect(foo: Foo) {}

fn main() {
    let _ = pass.into_function();

    let _ = too_many_arguments.into_function();
    //~^ E0599

    let _ = argument_not_reflect.into_function();
    //~^ E0599
}
