struct Input {
    one: bool,
    two: bool,
    three: bool,
}

fn inner(in: Input) {
    if (in.one && in.two) {
        discard;
    }
}

@fragment
fn outer(thing: bool, thing2: bool, thing3: bool) {
    var in: Input;
    in.one = thing;
    in.two = thing2;
    in.three = thing3;
    inner(in);
}