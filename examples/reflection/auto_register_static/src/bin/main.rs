//! Demonstrates how to set up automatic reflect types registration for platforms without `inventory` support
use auto_register_static::main as lib_main;
use bevy::reflect::load_type_registrations;

fn main() {
    // This must be called before our main to collect all type registration functions.
    load_type_registrations!();
    // After running load_type_registrations! we just forward to our main.
    lib_main();
}
