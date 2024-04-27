//! Tool used to build the templated pages of the Bevy website.

use bitflags::bitflags;

mod examples;
mod features;

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Command: u32 {
        const CHECK_MISSING = 0b00000001;
        const UPDATE = 0b00000010;
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Target: u32 {
        const EXAMPLES = 0b00000001;
        const FEATURES = 0b00000010;
    }
}

fn main() {
    let what_to_run = match std::env::args().nth(1).as_deref() {
        Some("check-missing") => Command::CHECK_MISSING,
        Some("update") => Command::UPDATE,
        _ => Command::all(),
    };

    let target = match std::env::args().nth(2).as_deref() {
        Some("examples") => Target::EXAMPLES,
        Some("features") => Target::FEATURES,
        _ => Target::all(),
    };

    if target.contains(Target::EXAMPLES) {
        examples::check(what_to_run);
    }
    if target.contains(Target::FEATURES) {
        features::check(what_to_run);
    }
}
