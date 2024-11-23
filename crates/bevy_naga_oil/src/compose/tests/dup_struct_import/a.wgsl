#define_import_path a
#import struct

fn a() -> struct::MyStruct {
    var s_a: struct::MyStruct;
    s_a.value = 1.0;
    return s_a;
}