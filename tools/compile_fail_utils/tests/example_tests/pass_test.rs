//@check-pass

// This code is expected to compile correctly.
fn correct_borrowing() {
    let x = String::new();
    let y = &x;

    println!("{x}");
    println!("{y}");
}
