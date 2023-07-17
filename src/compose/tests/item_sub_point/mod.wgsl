#define_import_path mod

struct Frag {
    fragment: f32,
}

fn fragment(f: Frag) -> f32 {
    return f.fragment * 2.0;
}