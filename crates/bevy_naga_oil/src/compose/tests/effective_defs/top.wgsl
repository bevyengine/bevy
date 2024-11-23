#define_import_path test_module
#import mod a, b, c

fn entry_point() -> f32 {
    return f32(a + b + c);
}