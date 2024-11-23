#define_import_path mod

virtual fn inner(arg: f32) -> f32 {
    return arg * 2.0;
}

fn outer() -> f32 {
    return inner(1.0);
}