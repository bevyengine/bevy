struct Vertex {
    a: bool,
    b: bool,
    c: bool,
}

struct Input {
    one: bool,
    two: bool,
    three: bool,
}

fn inner(in: Input) {
    if (in.one == in.two) {
        discard;
    }
}

@fragment
fn outer(v: Vertex) {
    var input: Input;

    input.one = v.a;
    input.two = v.b;
    input.three = v.c;

    inner(input);
}