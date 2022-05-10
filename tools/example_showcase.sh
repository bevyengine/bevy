#!/bin/bash

duration='2'
function wait_seconds() { perl -e 'alarm shift; exec @ARGV' "$@"; }
run_example() {
    cargo build --example "$1"
    wait_seconds "$duration" cargo run --example "$1"
}

example_list="$(cargo build --example 2>&1)"
example_list=${example_list//$'\n'/}
example_list="${example_list#error\: \"--example\" takes one argument.Available examples\: }"
echo "$example_list"
for example in $example_list
do
    echo "Running example: $example"
    run_example "$example"
done
