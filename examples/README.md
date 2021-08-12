# Examples for the bash-builtins crate

This directory contains examples for the crate. You can test with the following
commands.

```console
$ git clone https://github.com/ayosec/bash-builtins.rs.git

$ cd bash-builtins.rs

$ cargo build --release --examples

$ enable -f target/release/examples/libcounter.so counter

$ enable -f target/release/examples/libupcase.so upcase

$ upcase a λ
A Λ

$ counter
0

$ counter
1
```
