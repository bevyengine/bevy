pub fn snake_to_pascal_case(name: &str) -> String {
    let mut n = String::new();

    name.split('_').for_each(|s| {
        s.chars().enumerate().for_each(|(i, c)| {
            if i == 0 {
                n.extend(c.to_uppercase());
            } else {
                n.push(c);
            }
        });
    });

    n
}
