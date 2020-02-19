# Debugging

## Macro Debugging

* Print the final output of a macro using ```cargo rustc --profile=check -- -Zunstable-options --pretty=expanded```
* Print output during macro compilation using ```eprintln!("hi");```