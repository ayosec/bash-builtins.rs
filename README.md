# bash-builtins

A Rust crate to provide utilities to implement loadable builtins for bash.

See the [API documentation](https://docs.rs/bash-builtins) for a detailed
explanation on how to use this crate.

## Example

See the [`examples` directory](./examples/) for more examples.

```rust
//! Bash builtin to implement a counter.

use bash_builtins::{builtin_metadata, Args, Builtin, BuiltinOptions, Result};
use std::io::{stdout, Write};

builtin_metadata!(
    name = "counter",
    create = Counter::default,
    short_doc = "counter [-r] [-s value] [-a value]",
    long_doc = "
        Print a value, and increment it.

        Options:
          -r\tReset the value to 0.
          -s\tSet the counter to a specific value.
          -a\tIncrement the counter by a value.
    ",
);

#[derive(BuiltinOptions)]
enum Opt {
    #[opt = 'r']
    Reset,

    #[opt = 's']
    Set(isize),

    #[opt = 'a']
    Add(isize),
}

#[derive(Default)]
struct Counter(isize);

impl Builtin for Counter {
    fn call(&mut self, args: &mut Args) -> Result<()> {
        // No options: print the current value and increment it.
        if args.is_empty() {
            // Use writeln!() instead of println!() to avoid
            // panicking if stdout is closed.
            writeln!(stdout(), "{}", self.0)?;

            self.0 += 1;
            return Ok(());
        }

        // Parse options. They can change the value of the counter, but the
        // updated value is stored only if we don't get any error.
        let mut value = self.0;
        for opt in args.options() {
            match opt? {
                Opt::Reset => value = 0,
                Opt::Set(v) => value = v,
                Opt::Add(v) => value += v,
            }
        }

        // It is an error if we receive free arguments.
        args.finished()?;

        // Update the state and exit.
        self.0 = value;
        Ok(())
    }
}
```

# License

This crate is released under the terms of Apache License 2.0, which is
compatible with GPLv3.

See <https://www.apache.org/licenses/GPL-compatibility.html> for more
information about the compatibility.
