#!/bin/bash

duration='3'
run_example() {
    timeout "$duration" cargo run --release --example $1
}



for entry in examples/*
do
    IFS='/'
    read -ra ADDR <<< $entry
    IFS=' '
    example_file="${ADDR[1]}"
    if [ ${example_file: -2} == "rs" ]
    then
        example="${example_file::-3}"
        echo "Running example: $example"
        run_example $example
    fi 
done