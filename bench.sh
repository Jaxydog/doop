#! /bin/bash

cd ~/code/rust/doop
rm log/ -r
cargo b -r &&
hyperfine --warmup 5 --runs 100 'target/release/doop'