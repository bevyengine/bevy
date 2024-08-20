#![allow(unused)]

use bevy_reflect::func::{DynamicFunction, IntoFunction};
use bevy_reflect::Reflect;

fn main() {
    let value = String::from("Hello, World!");
    let closure_capture_owned = move || println!("{}", value);

    // Pass:
    let _: DynamicFunction<'static> = closure_capture_owned.into_function();

    let value = String::from("Hello, World!");
    let closure_capture_reference = || println!("{}", value);
    //~^ ERROR: `value` does not live long enough

    let _: DynamicFunction<'static> = closure_capture_reference.into_function();
}
