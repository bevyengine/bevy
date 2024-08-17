#![allow(unused)]

use bevy_reflect::func::{DynamicClosure, IntoClosure};
use bevy_reflect::Reflect;

fn main() {
    let value = String::from("Hello, World!");
    let closure_capture_owned = move || println!("{}", value);

    // Pass:
    let _: DynamicClosure<'static> = closure_capture_owned.into_closure();

    let value = String::from("Hello, World!");
    let closure_capture_reference = || println!("{}", value);
    //~^ ERROR: `value` does not live long enough

    let _: DynamicClosure<'static> = closure_capture_reference.into_closure();
}
