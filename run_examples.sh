#!/bin/bash

for file in examples/*; do
    if [ -f "$file" ]; then
        printf "Running $file: \n"
        cargo run --release "$file"
        printf "\n"
    fi
done
