// TODO: Make ShortTypeNames hold a set of dynamic allocated strings parking_lot = "0.11.1" + lazy_static
// struct ShortTypeNames {
//     types: Mutex<HashMap<TypeId, String, FnvBuildHasher>>,
// }

pub(crate) fn shorten_name(input: &str) -> String {
    // ? NOTE: Right for generic types
    let mut chars = input.chars().rev();
    let mut output = String::new();
    let mut depth = 0usize;
    let mut k = usize::MAX;
    while let Some(c) = chars.next() {
        if c == '>' {
            output.push('>');
            depth += 1;
        } else if c == '<' {
            output.push('<');
            depth -= 1;
        } else if c == ':' {
            if depth == 0 {
                break;
            }
            chars.next(); // skip next
            k = depth;
        } else if k != depth {
            output.push(c);
        }
    }
    // TODO: Find a better way that doesn't rely on yet another allocation
    output.chars().rev().collect()
}
