#define_import_path test_module

#import middle
#import mod

fn entry_point() -> f32 {
    return mod::outer();
}