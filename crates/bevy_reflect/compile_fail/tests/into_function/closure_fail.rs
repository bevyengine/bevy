//@no-rustfix
#![allow(unused)]

use bevy_reflect::func::IntoFunction;
use bevy_reflect::Reflect;

fn main() {
    let value = String::from("Hello, World!");
    let closure_capture_owned = move || println!("{}", value);

    // Should pass:
    let _ = closure_capture_owned.into_function();

    let value = String::from("Hello, World!");
    let closure_capture_reference = || println!("{}", &value);
    //~^ E0373

    // Above error due to this line:
    let _ = closure_capture_reference.into_function();
}
