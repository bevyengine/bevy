const PI: f32 = 3.1;

fn b__f() -> f32 {
    return PI * 2.0;
}

fn b__g() -> f32 {
    return PI * 2.0;
}

fn a__f() -> f32 {
    return PI * 1.0;
}

fn main() -> f32 {
    let x = a__f();
    let y = b__f();

    return x*y;
} 