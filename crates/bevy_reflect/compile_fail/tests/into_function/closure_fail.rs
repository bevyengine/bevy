#![allow(unused)]

use bevy_reflect::func::IntoFunction;
use bevy_reflect::Reflect;

fn main() {
    let value = String::from("Hello, World!");
    let closure_capture_owned = move || println!("{}", value);

    let _ = closure_capture_owned.into_function();
    //~^ E0277

    let value = String::from("Hello, World!");
    let closure_capture_reference = || println!("{}", value);

    let _ = closure_capture_reference.into_function();
    // ↑ This should be an error (E0277) but `compile_fail_utils` fails to pick it up
    // when the `closure_capture_owned` test is present
}
