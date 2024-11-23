#import mod

override fn mod::outer() -> f32 {
    return 99.0;
}

fn top() -> f32 {
    return mod::outer();
}