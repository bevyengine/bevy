#define_import_path a
#import struct

fn a() -> struct::MyStruct {
    var s: struct::MyStruct;
    s.value = 1.0;
    return s;
}