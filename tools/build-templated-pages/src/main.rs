use bitflags::bitflags;

mod examples;

bitflags! {
    struct Command: u32 {
        const CHECK_MISSING = 0b00000001;
        const UPDATE = 0b00000010;
    }
}

bitflags! {
    struct Target: u32 {
        const EXAMPLES = 0b00000001;
    }
}

fn main() {
    let what_to_run = match std::env::args().nth(1).as_deref() {
        Some("check-missing") => Command::CHECK_MISSING,
        Some("update") => Command::UPDATE,
        _ => Command::all(),
    };

    let target = match std::env::args().nth(2).as_deref() {
        Some("exmaples") => Target::EXAMPLES,
        _ => Target::all(),
    };

    if target.contains(Target::EXAMPLES) {
        examples::check(what_to_run);
    }
}
