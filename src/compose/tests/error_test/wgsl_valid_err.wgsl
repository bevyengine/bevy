#define_import_path valid_inc

fn ok() -> f32 {
    return 1.0;
}

fn func() -> f32 {
    return 1u;
}

fn still_ok() -> f32 {
    return 1.0;
}

fn main() {
    let x: f32 = func();
}