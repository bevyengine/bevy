// Compiler warnings also need to be annotated. We don't
// want to annotate all the unused variables so let's instruct
// the compiler to ignore them.
#![allow(unused_variables)]

fn bad_moves() {
    let x = String::new();
    // Help diagnostics need to be annotated
    let y = x;
    //~^ HELP: consider cloning

    // We expect a failure on this line
    println!("{x}"); //~ ERROR: borrow


    let x = String::new();
    // We expect the help message to mention cloning.
    //~v HELP: consider cloning
    let y = x;

    // Check error message using a regex
    println!("{x}");
    //~^ ERROR: /(move)|(borrow)/
}
