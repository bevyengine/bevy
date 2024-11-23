#import mod

fn add() {
    mod::a += 1.0;
}

fn main() -> f32 {
    add();
    add();
    return mod::a;
}