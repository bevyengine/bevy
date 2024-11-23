#import "quoted_module" as foo;

fn myfunc(foo: u32) -> f32 {
    return f32(foo) * 2.0; 
}

fn main() -> f32 {
    return myfunc(1u) + foo::foo();
}