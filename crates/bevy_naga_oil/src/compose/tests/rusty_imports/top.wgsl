#define_import_path test_module

#import a::b as partial_path
#import a::b::c as full_path

fn entry_point() -> f32 {
    return a::x::square(partial_path::c::triple(full_path::C));
}