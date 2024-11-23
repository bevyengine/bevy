#import test_module

@group(0) @binding(0) 
var<storage, read_write> buffer: f32;

@compute @workgroup_size(1, 1, 1)
fn run_test() {
    let res = test_module::entry_point();
    buffer = res;
}
